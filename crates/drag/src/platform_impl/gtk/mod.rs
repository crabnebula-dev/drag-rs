// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{CursorPosition, DragItem, DragResult, Error, Image, Options};
use gdkx11::{
    gdk,
    glib::{ObjectExt, Propagation, SignalHandlerId},
};
use gtk::{
    gdk_pixbuf,
    prelude::{
        DeviceExt, DragContextExtManual, PixbufLoaderExt, SeatExt, WidgetExt, WidgetExtManual,
    },
};
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

pub fn start_drag<F: Fn(DragResult, CursorPosition) + Send + 'static>(
    window: &gtk::ApplicationWindow,
    item: DragItem,
    image: Image,
    on_drop_callback: F,
    options: Options,
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
            // Currently leaving it as is as we can utilize it as a dummy dragging feature
            // on_drop_callback(DragResult::Cancel, get_cursor_position(window).unwrap());
            // return Ok(());
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
            on_drop_failed(callback.clone(), window, &handler_ids, &options);
            on_drop_performed(callback.clone(), window, &handler_ids, &drag_context);

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

fn on_drop_failed<F: Fn(DragResult, CursorPosition) + Send + 'static>(
    callback: Rc<F>,
    window: &gtk::ApplicationWindow,
    handler_ids: &Arc<Mutex<Vec<SignalHandlerId>>>,
    options: &Options,
) {
    let window_clone = window.clone();
    let handler_ids_clone = handler_ids.clone();

    let skip_animatation_on_cancel_or_failure = options.skip_animatation_on_cancel_or_failure;

    handler_ids
        .lock()
        .unwrap()
        .push(window.connect_drag_failed(move |_, _, _drag_result| {
            callback(
                DragResult::Cancel,
                get_cursor_position(&window_clone).unwrap(),
            );

            cleanup_signal_handlers(&handler_ids_clone, &window_clone);
            if skip_animatation_on_cancel_or_failure {
                Propagation::Stop
            } else {
                Propagation::Proceed
            }
        }));
}

fn cleanup_signal_handlers(
    handler_ids: &Arc<Mutex<Vec<SignalHandlerId>>>,
    window: &gtk::ApplicationWindow,
) {
    let handler_ids = &mut handler_ids.lock().unwrap();
    clear_signal_handlers(window, handler_ids);
    window.drag_source_unset();
}

fn on_drop_performed<F: Fn(DragResult, CursorPosition) + Send + 'static>(
    callback: Rc<F>,
    window: &gtk::ApplicationWindow,
    handler_ids: &Arc<Mutex<Vec<SignalHandlerId>>>,
    drag_context: &gdk::DragContext,
) {
    let window = window.clone();
    let handler_ids = handler_ids.clone();

    drag_context.connect_drop_performed(move |_, _| {
        cleanup_signal_handlers(&handler_ids, &window);
        callback(DragResult::Dropped, get_cursor_position(&window).unwrap());
    });
}

fn get_cursor_position(window: &gtk::ApplicationWindow) -> Result<CursorPosition, Error> {
    if let Some(cursor) = window
        .display()
        .default_seat()
        .and_then(|seat| seat.pointer())
    {
        let (_, x, y) = cursor.position();
        Ok(CursorPosition { x, y })
    } else {
        Err(Error::FailedToGetCursorPosition)
    }
}
