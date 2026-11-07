"""Read-only Windows display/window facts for the owned app PID. No automation."""

import ctypes
from ctypes import wintypes as w
import re

from native_refresh import refresh


class DisplayDevice(ctypes.Structure):
    _fields_ = [("cb", w.DWORD), ("name", w.WCHAR * 32), ("description", w.WCHAR * 128),
                ("flags", w.DWORD), ("device_id", w.WCHAR * 128), ("key", w.WCHAR * 128)]


class DisplayMode(ctypes.Structure):
    # The display branch of DEVMODEW's 16-byte printer/display union.
    _fields_ = [("name", w.WCHAR * 32), ("spec", w.WORD), ("driver", w.WORD),
                ("size", w.WORD), ("extra", w.WORD), ("fields", w.DWORD),
                ("position", w.POINT), ("orientation", w.DWORD), ("fixed", w.DWORD),
                ("color", w.SHORT), ("duplex", w.SHORT), ("y_resolution", w.SHORT),
                ("tt", w.SHORT), ("collate", w.SHORT), ("form", w.WCHAR * 32),
                ("dpi", w.WORD), ("bits", w.DWORD), ("width", w.DWORD),
                ("height", w.DWORD), ("display_flags", w.DWORD), ("hz", w.DWORD)] + [
                    (key, w.DWORD) for key in ("icm_method", "icm_intent", "media", "dither",
                                              "reserved1", "reserved2", "pan_width", "pan_height")]


class MonitorInfo(ctypes.Structure):
    _fields_ = [("size", w.DWORD), ("monitor", w.RECT), ("work", w.RECT),
                ("flags", w.DWORD), ("device", w.WCHAR * 32)]


def signature(dll, name, args, result):
    function = getattr(dll, name)
    function.argtypes, function.restype = args, result
    return function


def driver_version(device):
    """Registry version for the active GDI adapter, not a remembered driver name."""
    import winreg
    prefix = "\\Registry\\Machine\\"
    if not device.key.lower().startswith(prefix.lower()):
        raise OSError("display device has no machine registry provenance")
    path = device.key[len(prefix):]
    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, path) as key:
        version, _ = winreg.QueryValueEx(key, "DriverVersion")
    if not isinstance(version, str) or not re.fullmatch(r"[0-9]+(?:\.[0-9]+)+", version):
        raise OSError("native adapter DriverVersion is missing or malformed")
    return {"version": version, "source": "HKLM\\" + path + "\\DriverVersion",
            "device_id": device.device_id, "description": device.description}


class Display:
    def __init__(self):
        self.user = ctypes.WinDLL("user32", use_last_error=True)
        self.callback_type = ctypes.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
        self.enumerate = signature(self.user, "EnumWindows", [self.callback_type, w.LPARAM], w.BOOL)
        self.owner = signature(self.user, "GetWindowThreadProcessId", [w.HWND, ctypes.POINTER(w.DWORD)], w.DWORD)
        self.visible = signature(self.user, "IsWindowVisible", [w.HWND], w.BOOL)
        self.client = signature(self.user, "GetClientRect", [w.HWND, ctypes.POINTER(w.RECT)], w.BOOL)
        self.monitor = signature(self.user, "MonitorFromWindow", [w.HWND, w.DWORD], w.HANDLE)
        self.monitor_info = signature(self.user, "GetMonitorInfoW", [w.HANDLE, ctypes.POINTER(MonitorInfo)], w.BOOL)
        self.mode = signature(self.user, "EnumDisplaySettingsExW", [w.LPCWSTR, w.DWORD, ctypes.POINTER(DisplayMode), w.DWORD], w.BOOL)
        self.device = signature(self.user, "EnumDisplayDevicesW", [w.LPCWSTR, w.DWORD, ctypes.POINTER(DisplayDevice), w.DWORD], w.BOOL)

    def sample(self, pid):
        windows = []

        @self.callback_type
        def visit(handle, _):
            owner = w.DWORD()
            self.owner(handle, ctypes.byref(owner))
            if owner.value == pid and self.visible(handle):
                rect = w.RECT()
                if self.client(handle, ctypes.byref(rect)) and rect.right > rect.left and rect.bottom > rect.top:
                    windows.append((handle, [rect.right - rect.left, rect.bottom - rect.top]))
            return True

        if not self.enumerate(visit, 0):
            raise ctypes.WinError(ctypes.get_last_error())
        if len(windows) != 1:
            raise OSError("owned process does not yet have exactly one visible client window")
        handle, client = windows[0]
        monitor = self.monitor(handle, 0)
        info = MonitorInfo()
        info.size = ctypes.sizeof(info)
        if not monitor or not self.monitor_info(monitor, ctypes.byref(info)):
            raise OSError("owned app monitor is unavailable")
        mode = DisplayMode()
        mode.size = ctypes.sizeof(mode)
        if not self.mode(info.device, 0xFFFFFFFF, ctypes.byref(mode), 0) or mode.hz <= 1:
            raise OSError("current app monitor mode/refresh is unavailable")
        device = DisplayDevice()
        found = False
        for index in range(64):
            device.cb = ctypes.sizeof(device)
            if not self.device(None, index, ctypes.byref(device), 0):
                break
            if device.name == info.device:
                found = True
                break
        if not found:
            raise OSError("app monitor has no native display-adapter mapping")
        try:
            driver = {"available": True, **driver_version(device)}
        except OSError as exc:
            driver = {"available": False, "reason": str(exc)}
        rational = refresh(self.user, info.device)
        return {"available": True, "source": "EnumWindows/GetWindowThreadProcessId/GetClientRect/MonitorFromWindow/GetMonitorInfoW/EnumDisplaySettingsExW/EnumDisplayDevicesW",
                "monitor_device": info.device, "desktop_pixels": [mode.width, mode.height],
                "client_pixels": client, "refresh_hz": rational["hz"], "refresh_rational": rational,
                "gdi_integer_refresh_hz": mode.hz,
                "orientation": mode.orientation, "bits_per_pixel": mode.bits,
                "adapter_device_id": device.device_id, "driver": driver}
