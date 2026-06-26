"""Project manager service helpers — extracted from editor/server/main.py."""

from pathlib import Path


def normalize_path(p: str) -> str:
    """Normalize a file path, handling POSIX-style paths on Windows
    (e.g. /d/space/... -> D:/space/...).
    """
    path = Path(p)
    resolved = path.resolve()
    # If resolved path doesn't exist and it looks like a POSIX absolute path (/x/...),
    # try converting the first component to a drive letter
    if not resolved.exists() and p.startswith("/") and len(p) > 2 and p[2] == "/":
        drive_letter = p[1].upper()
        alt = Path(f"{drive_letter}:/{p[3:]}").resolve()
        if alt.exists():
            return str(alt)
    return str(resolved)
