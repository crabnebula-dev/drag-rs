---
drag: patch
---

Validate the file paths passed to a drag operation on Windows and return the new `Error::InvalidShellPath` variant instead of failing silently when a path cannot be resolved to a shell item.
