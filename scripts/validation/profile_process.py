"""Windows process observations and bounded child ownership for #1087.

No shell evaluation. These helpers observe only the process started by the
caller. Working set is resident process memory, not GPU or total system memory.
"""

import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import subprocess
import time


class MemoryCounters(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
        (name, ctypes.c_size_t) for name in (
            "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
            "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage",
            "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage",
        )
    ]


class WorkingSet:
    """Own a query handle, not the app lifetime; never signal arbitrary PIDs."""

    def __init__(self, pid):
        if os.name != "nt":
            raise OSError("working-set sampling is supported on Windows only")
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.psapi = ctypes.WinDLL("psapi", use_last_error=True)
        self.kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        self.kernel.OpenProcess.restype = wintypes.HANDLE
        self.kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        self.kernel.CloseHandle.restype = wintypes.BOOL
        self.psapi.GetProcessMemoryInfo.argtypes = [
            wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD,
        ]
        self.psapi.GetProcessMemoryInfo.restype = wintypes.BOOL
        # PROCESS_QUERY_INFORMATION | PROCESS_VM_READ. No termination right.
        self.handle = self.kernel.OpenProcess(0x0400 | 0x0010, False, pid)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())

    def sample(self):
        counters = MemoryCounters()
        counters.cb = ctypes.sizeof(counters)
        if not self.psapi.GetProcessMemoryInfo(self.handle, ctypes.byref(counters), counters.cb):
            raise ctypes.WinError(ctypes.get_last_error())
        return counters.WorkingSetSize

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None


def physical_ram():
    """Installed physical capacity as reported by GlobalMemoryStatusEx."""
    class Status(ctypes.Structure):
        _fields_ = [("length", wintypes.DWORD), ("load", wintypes.DWORD)] + [
            (name, ctypes.c_ulonglong) for name in (
                "total", "available", "page_total", "page_available",
                "virtual_total", "virtual_available", "extended_available",
            )
        ]
    if os.name != "nt":
        return None
    status = Status()
    status.length = ctypes.sizeof(status)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.GlobalMemoryStatusEx.argtypes = [ctypes.POINTER(Status)]
    kernel.GlobalMemoryStatusEx.restype = wintypes.BOOL
    if not kernel.GlobalMemoryStatusEx(ctypes.byref(status)):
        raise ctypes.WinError(ctypes.get_last_error())
    return status.total


def write_json(path, value):
    """Write observations after execution, never overwrite unrelated evidence."""
    Path(path).write_text(json.dumps(value, indent=2, allow_nan=False) + "\n", encoding="utf-8")


def observe(argv, directory, cwd, environment, timeout, collect=True):
    """Run one direct child, stream evidence, and reap it on every exit path.

    argv is built by the caller, never supplied as a shell string. No compiler
    wrapper: the PID being sampled must be the actual app PID. A missing sample
    remains missing and a sampler failure makes the run non-successful.
    """
    directory = Path(directory)
    result = {"status": "launch-failed", "pid": None, "exit_code": None,
              "elapsed_s": 0.0, "samples": 0, "observed_peak_working_set_bytes": None,
              "sample_period_s": 1.0, "sampling_enabled": collect,
              "sampling_gaps": [], "problems": []}
    child = None
    memory = None
    start = time.monotonic()
    try:
        with (directory / "stdout.txt").open("w", encoding="utf-8") as stdout, \
                (directory / "stderr.txt").open("w", encoding="utf-8") as stderr, \
                (directory / "memory.jsonl").open("w", encoding="utf-8") as samples:
            child = subprocess.Popen(argv, cwd=cwd, env=environment, stdout=stdout,
                                     stderr=stderr, stdin=subprocess.DEVNULL, shell=False)
            result["pid"] = child.pid
            result["status"] = "running"
            # Persist ownership before waiting, including for interrupted runs.
            write_json(directory / "process.json", result)
            if collect:
                memory = WorkingSet(child.pid)
            due = start
            previous = None
            while True:
                now = time.monotonic()
                code = child.poll()
                if code is not None:
                    result["exit_code"] = code
                    result["status"] = "completed" if code == 0 else "process-failed"
                    break
                if now - start >= timeout:
                    result["status"] = "timeout"
                    result["problems"].append("child exceeded the configured timeout")
                    break
                if memory is not None and now >= due:
                    elapsed = now - start
                    if previous is not None and elapsed - previous > 1.5:
                        result["sampling_gaps"].append({"from_s": previous, "to_s": elapsed})
                    try:
                        value = memory.sample()
                    except OSError as exc:
                        # A natural exit between poll and observation is not an API failure.
                        if child.poll() is not None:
                            continue
                        result["status"] = "sampling-failed"
                        result["problems"].append(str(exc))
                        break
                    samples.write(json.dumps({"elapsed_s": elapsed, "pid": child.pid,
                                              "working_set_bytes": value}) + "\n")
                    samples.flush()
                    result["samples"] += 1
                    result["observed_peak_working_set_bytes"] = max(
                        value, result["observed_peak_working_set_bytes"] or 0)
                    previous = elapsed
                    due = now + 1.0
                # Responsive timeout/exit detection; never spin or generate fake samples.
                time.sleep(min(0.1, max(0.001, timeout - (time.monotonic() - start))))
    except (OSError, ValueError) as exc:
        result["status"] = "sampling-failed" if child is not None else "launch-failed"
        result["problems"].append(str(exc))
    except KeyboardInterrupt:
        result["status"] = "interrupted"
        result["problems"].append("operator interrupted the run")
    finally:
        if memory is not None:
            memory.close()
        if child is not None:
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)
            result["exit_code"] = child.returncode
        result["elapsed_s"] = time.monotonic() - start
        if collect and result["status"] == "completed" and not result["samples"]:
            result["status"] = "sampling-failed"
            result["problems"].append("no working-set sample was obtained")
        write_json(directory / "process.json", result)
    return result
