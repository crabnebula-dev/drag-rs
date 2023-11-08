use raw_window_handle::HasRawWindowHandle;

use crate::{DragItem, Image};

pub fn start_drag<W: HasRawWindowHandle>(handle: &W, item: DragItem, image: Image) {}
