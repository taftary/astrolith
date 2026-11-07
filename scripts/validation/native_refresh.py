"""Exact nominal refresh rational for the current active Windows display path."""

import ctypes
from ctypes import wintypes as w


class Luid(ctypes.Structure):
    _fields_ = [("low", w.DWORD), ("high", w.LONG)]


class Rational(ctypes.Structure):
    _fields_ = [("numerator", w.UINT), ("denominator", w.UINT)]


class PathSource(ctypes.Structure):
    _fields_ = [("adapter", Luid), ("id", w.UINT), ("mode", w.UINT), ("flags", w.UINT)]


class PathTarget(ctypes.Structure):
    _fields_ = [("adapter", Luid), ("id", w.UINT), ("mode", w.UINT), ("technology", w.UINT),
                ("rotation", w.UINT), ("scaling", w.UINT), ("refresh", Rational),
                ("scanline", w.UINT), ("available", w.BOOL), ("flags", w.UINT)]


class DisplayPath(ctypes.Structure):
    _fields_ = [("source", PathSource), ("target", PathTarget), ("flags", w.UINT)]


class ModePayload(ctypes.Union):
    # DISPLAYCONFIG_MODE_INFO's largest branch is 48 bytes, aligned to UINT64.
    _fields_ = [("video_signal", ctypes.c_ulonglong * 6)]


class ModeInfo(ctypes.Structure):
    _fields_ = [("kind", w.UINT), ("id", w.UINT), ("adapter", Luid), ("payload", ModePayload)]


class SourceName(ctypes.Structure):
    _fields_ = [("kind", w.UINT), ("size", w.UINT), ("adapter", Luid), ("id", w.UINT),
                ("device", w.WCHAR * 32)]


def refresh(user, device):
    """Read QDC_ONLY_ACTIVE_PATHS, resolving source names rather than guessing monitor order."""
    sizes, query, name = user.GetDisplayConfigBufferSizes, user.QueryDisplayConfig, user.DisplayConfigGetDeviceInfo
    sizes.argtypes, sizes.restype = [w.UINT, ctypes.POINTER(w.UINT), ctypes.POINTER(w.UINT)], w.LONG
    query.argtypes = [w.UINT, ctypes.POINTER(w.UINT), ctypes.POINTER(DisplayPath), ctypes.POINTER(w.UINT), ctypes.POINTER(ModeInfo), ctypes.c_void_p]
    query.restype = w.LONG
    name.argtypes, name.restype = [ctypes.POINTER(SourceName)], w.LONG
    for _ in range(3):
        paths_count, modes_count = w.UINT(), w.UINT()
        result = sizes(2, ctypes.byref(paths_count), ctypes.byref(modes_count))
        if result or not 0 < paths_count.value <= 64 or not 0 < modes_count.value <= 256:
            raise OSError(f"active display buffer sizes unavailable/excessive ({result})")
        paths, modes = (DisplayPath * paths_count.value)(), (ModeInfo * modes_count.value)()
        result = query(2, ctypes.byref(paths_count), paths, ctypes.byref(modes_count), modes, None)
        if result == 122:
            continue
        if result:
            raise OSError(f"active display path query returned {result}")
        matches = []
        for path in paths[:paths_count.value]:
            source = SourceName()
            source.kind, source.size = 1, ctypes.sizeof(source)
            source.adapter, source.id = path.source.adapter, path.source.id
            result = name(ctypes.byref(source))
            if result:
                raise OSError(f"display source name query returned {result}")
            if source.device == device:
                matches.append(path.target.refresh)
        if len(matches) != 1 or not matches[0].numerator or not matches[0].denominator:
            raise OSError("app monitor has no unique nonzero refresh rational")
        rational = matches[0]
        return {"numerator": rational.numerator, "denominator": rational.denominator,
                "hz": rational.numerator / rational.denominator,
                "source": "GetDisplayConfigBufferSizes/QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS)/DisplayConfigGetDeviceInfo(source GDI name)",
                "precision": "current Windows path nominal rational Hz, not physical scan-out timing"}
    raise OSError("active display topology changed during all three bounded queries")
