---
"drag": patch
---

Drag files from network shares and FUSE-backed volumes on Windows. Absolute paths are no longer canonicalized: `dunce` keeps the verbatim `\\?\` form for network paths and for anything over `MAX_PATH`, which the shell's namespace parser cannot read, and it fails outright on rclone, Cryptomator and WinFsp volumes. A verbatim path handed in by a caller is converted before it is parsed.
