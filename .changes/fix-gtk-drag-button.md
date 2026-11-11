---
"drag": patch
---

Fix drags never dropping on Linux by passing the mouse button number instead of `GDK_BUTTON1_MASK` to `drag_begin_with_coordinates`.
