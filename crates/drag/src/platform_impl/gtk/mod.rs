// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{DragItem, Image};
use gdkx11::gdk;
use gtk::{
    gdk_pixbuf,
    prelude::{DragContextExtManual, PixbufLoaderExt, WidgetExt, WidgetExtManual},
};

pub fn start_drag(
    window: &gtk::ApplicationWindow,
    item: DragItem,
    image: Image,
) -> crate::Result<()> {
    window.drag_source_set(gdk::ModifierType::BUTTON1_MASK, &[], gdk::DragAction::COPY);

    match item {
        DragItem::Files(paths) => {
            window.drag_source_add_uri_targets();

            window.connect_drag_data_get(move |_, _, data, _, _| {
                let uris: Vec<String> = paths
                    .iter()
                    .map(|path| format!("file://{}", path.display()))
                    .collect();
                let uris: Vec<&str> = uris.iter().map(|s| s.as_str()).collect();
                data.set_uris(&uris);
            });
        }
    }

    window.connect_drag_end(|this, _| {
        this.drag_source_unset();
    });

    if let Some(target_list) = &window.drag_source_get_target_list() {
        if let Some(drag_context) = window.drag_begin_with_coordinates(
            target_list,
            gdk::DragAction::COPY,
            gdk::ffi::GDK_BUTTON1_MASK as i32,
            None,
            -1,
            -1,
        ) {
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
