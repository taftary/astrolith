"""Bounded run-time Windows conditions. Unknown facts stay explicitly unavailable."""

import ctypes
from ctypes import wintypes as w
import os
import platform

from native_display import Display, signature
from profile_process import PreciseWallClock, physical_ram


class Guid(ctypes.Structure):
    _fields_ = [("a", w.DWORD), ("b", w.WORD), ("c", w.WORD), ("d", ctypes.c_ubyte * 8)]

    def text(self):
        data = bytes(self.d).hex()
        return f"{self.a:08x}-{self.b:04x}-{self.c:04x}-{data[:4]}-{data[4:]}"


class PowerStatus(ctypes.Structure):
    _fields_ = [(key, w.BYTE) for key in ("ac", "battery_flags", "percent", "saver")] + [
        ("lifetime_s", w.DWORD), ("full_lifetime_s", w.DWORD)]


def unavailable(call):
    try:
        return call()
    except (OSError, AttributeError, ValueError) as exc:
        return {"available": False, "reason": str(exc)}


class Conditions:
    """No PID selection by the caller: the runner supplies only its direct child."""

    def __init__(self):
        if os.name != "nt":
            raise OSError("native comparison conditions currently require Windows")
        self.wall = PreciseWallClock()
        self.display = Display()
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.powrprof = ctypes.WinDLL("powrprof", use_last_error=True)
        self.power_status = signature(self.kernel, "GetSystemPowerStatus", [ctypes.POINTER(PowerStatus)], w.BOOL)
        self.active = signature(self.powrprof, "PowerGetActiveScheme", [w.HANDLE, ctypes.POINTER(ctypes.POINTER(Guid))], w.DWORD)
        self.free = signature(self.kernel, "LocalFree", [w.HANDLE], w.HANDLE)
        self.overlay = (signature(self.powrprof, "PowerGetEffectiveOverlayScheme", [ctypes.POINTER(Guid)], w.DWORD)
                        if hasattr(self.powrprof, "PowerGetEffectiveOverlayScheme") else None)

    def scheme(self, overlay=False):
        if overlay:
            if self.overlay is None:
                raise OSError("native effective power overlay API is unavailable")
            guid = Guid()
            result = self.overlay(ctypes.byref(guid))
            if result:
                raise OSError(f"native effective overlay query returned {result}")
            return guid.text()
        pointer = ctypes.POINTER(Guid)()
        result = self.active(None, ctypes.byref(pointer))
        if result or not pointer:
            raise OSError(f"native power scheme query returned {result}")
        try:
            return pointer.contents.text()
        finally:
            self.free(ctypes.cast(pointer, w.HANDLE))

    def power(self):
        status = PowerStatus()
        if not self.power_status(ctypes.byref(status)):
            raise ctypes.WinError(ctypes.get_last_error())
        if status.ac == 255 or status.battery_flags == 255:
            raise OSError("native AC/battery condition is unknown")
        return {"available": True, "source": "GetSystemPowerStatus/PowerGetActiveScheme/PowerGetEffectiveOverlayScheme",
                "ac_line": status.ac, "battery_flags": status.battery_flags, "battery_saver": status.saver,
                "battery_percent": status.percent if status.percent != 255 else None,
                "active_scheme": self.scheme(), "effective_overlay": self.scheme(overlay=True)}

    def sample(self, pid):
        before = self.wall.sample_ns()
        display = unavailable(lambda: self.display.sample(pid))
        power = unavailable(self.power)
        return {"schema": "native-comparison-conditions-v1", "pid": pid,
                "utc_before_ns": before, "utc_after_ns": self.wall.sample_ns(),
                "display": display, "power": power}


def machine():
    def cpu():
        import winreg
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0") as key:
            name, _ = winreg.QueryValueEx(key, "ProcessorNameString")
        return {"available": True, "name": name.strip(), "logical_cpu_count": os.cpu_count(),
                "physical_ram_bytes": physical_ram(), "platform": platform.platform(),
                "source": "native Windows CPU registry/GlobalMemoryStatusEx/Python platform"}
    return unavailable(cpu)
