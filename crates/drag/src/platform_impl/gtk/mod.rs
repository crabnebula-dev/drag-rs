// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::{DragItem, DragResult, Image};
use gdkx11::{
    gdk,
    glib::{ObjectExt, SignalHandlerId},
};
use gtk::{
    gdk_pixbuf,
    prelude::{DragContextExtManual, PixbufLoaderExt, WidgetExt, WidgetExtManual},
};

pub fn start_drag<F: Fn(DragResult) + Send + 'static>(
    window: &gtk::ApplicationWindow,
    item: DragItem,
    image: Image,
    on_drop_callback: F,
) -> crate::Result<()> {
    let handler_ids: Arc<Mutex<Vec<SignalHandlerId>>> = Arc::new(Mutex::new(vec![]));

    window.drag_source_set(gdk::ModifierType::BUTTON1_MASK, &[], gdk::DragAction::COPY);

    match item {
        DragItem::Files(paths) => {
            window.drag_source_add_uri_targets();
            handler_ids
                .lock()
                .unwrap()
                .push(window.connect_drag_data_get(move |_, _, data, _, _| {
                    let uris: Vec<String> = paths
                        .iter()
                        .map(|path| format!("file://{}", path.display()))
                        .collect();
                    let uris: Vec<&str> = uris.iter().map(|s| s.as_str()).collect();
                    data.set_uris(&uris);
                }));
        }
        DragItem::Data { .. } => {
            on_drop_callback(DragResult::Cancel);
            return Ok(());
        }
    }

    if let Some(target_list) = &window.drag_source_get_target_list() {
        if let Some(drag_context) = window.drag_begin_with_coordinates(
            target_list,
            gdk::DragAction::COPY,
            gdk::ffi::GDK_BUTTON1_MASK as i32,
            None,
            -1,
            -1,
        ) {
            let callback = Rc::new(on_drop_callback);
            on_drop_cancel(callback.clone(), window, &handler_ids, &drag_context);
            on_drop_performed(callback, window, &handler_ids, &drag_context);

            let icon_pixbuf: Option<gdk_pixbuf::Pixbuf> = match &image {
                Image::Raw(data) => image_binary_to_pixbuf(data),
                Image::File(path) => match std::fs::read(path) {
                    Ok(bytes) => image_binary_to_pixbuf(&bytes),
                    Err(_) => None,
                },
            };
            if let Some(icon) = icon_pixbuf {
                drag_context.drag_set_icon_pixbuf(&icon, 0, 0);
            }

            Ok(())
        } else {
            Err(crate::Error::FailedToStartDrag)
        }
    } else {
        Err(crate::Error::EmptyTargetList)
    }
}

fn image_binary_to_pixbuf(data: &[u8]) -> Option<gdk_pixbuf::Pixbuf> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader
        .write(data)
        .and_then(|_| loader.close())
        .map_err(|_| ())
        .and_then(|_| loader.pixbuf().ok_or(()))
        .ok()
}

fn clear_signal_handlers(window: &gtk::ApplicationWindow, handler_ids: &mut Vec<SignalHandlerId>) {
    for handler_id in handler_ids.drain(..) {
        window.disconnect(handler_id);
    }
}

fn on_drop_cancel<F: Fn(DragResult) + Send + 'static>(
    callback: Rc<F>,
    window: &gtk::ApplicationWindow,
    handler_ids: &Arc<Mutex<Vec<SignalHandlerId>>>,
    drag_context: &gdk::DragContext,
) {
    let window = window.clone();
    let handler_ids = handler_ids.clone();

    drag_context.connect_cancel(move |_, _| {
        let handler_ids = &mut handler_ids.lock().unwrap();
        clear_signal_handlers(&window, handler_ids);
        window.drag_source_unset();

        callback(DragResult::Cancel);
    });
}

fn on_drop_performed<F: Fn(DragResult) + Send + 'static>(
    callback: Rc<F>,
    window: &gtk::ApplicationWindow,
    handler_ids: &Arc<Mutex<Vec<SignalHandlerId>>>,
    drag_context: &gdk::DragContext,
) {
    let window = window.clone();
    let handler_ids = handler_ids.clone();

    drag_context.connect_drop_performed(move |_, _| {
        let handler_ids = &mut handler_ids.lock().unwrap();
        clear_signal_handlers(&window, handler_ids);
        window.drag_source_unset();

        callback(DragResult::Dropped);
    });
}
