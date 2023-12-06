// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use cocoa::{
    appkit::{NSAlignmentOptions, NSApp, NSEvent, NSEventModifierFlags, NSEventType, NSImage},
    base::{id, nil},
    foundation::{NSData, NSPoint, NSRect, NSSize, NSUInteger},
};
use objc::{
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use crate::{DragItem, Image};

const UTF8_ENCODING: usize = 4;

unsafe fn new_nsstring(s: &str) -> id {
    let ns_string: id = msg_send![class!(NSString), alloc];
    let ns_string: id =
        msg_send![ns_string, initWithBytes:s.as_ptr() length:s.len() encoding:UTF8_ENCODING];

    // The thing is allocated in rust, the thing must be set to autorelease in rust to relinquish control
    // or it can not be released correctly in OC runtime
    let _: () = msg_send![ns_string, autorelease];

    ns_string
}

pub fn start_drag<W: HasRawWindowHandle>(
    handle: &W,
    item: DragItem,
    image: Image,
) -> crate::Result<()> {
    if let RawWindowHandle::AppKit(w) = handle.raw_window_handle() {
        unsafe {
            let window = w.ns_window as id;
            // wry replaces the ns_view so we don't really use AppKitWindowHandle::ns_view
            let ns_view: id = msg_send![window, contentView];

            let mouse_location: NSPoint = msg_send![window, mouseLocationOutsideOfEventStream];
            let current_position: NSPoint = msg_send![ns_view, backingAlignedRect: NSRect::new(mouse_location, NSSize::new(0., 0.)) options: NSAlignmentOptions::NSAlignAllEdgesOutward];

            let img: id = msg_send![class!(NSImage), alloc];
            let img: id = match image {
                Image::File(path) => {
                    if !path.exists() {
                        return Err(crate::Error::ImageNotFound);
                    }
                    NSImage::initByReferencingFile_(img, new_nsstring(&path.to_string_lossy()))
                }
                Image::Raw(bytes) => {
                    let data = NSData::dataWithBytes_length_(
                        nil,
                        bytes.as_ptr() as *const std::os::raw::c_void,
                        bytes.len() as u64,
                    );
                    NSImage::initWithData_(NSImage::alloc(nil), data)
                }
            };
            let image_size: NSSize = img.size();
            let image_rect = NSRect::new(
                NSPoint::new(
                    current_position.x - image_size.width / 2.,
                    current_position.y - image_size.height / 2.,
                ),
                image_size,
            );

            let file_items: id = msg_send![class!(NSMutableArray), array];

            match item {
                DragItem::Files(files) => {
                    for path in files {
                        let nsurl: id = msg_send![class!(NSURL), fileURLWithPath: new_nsstring(&path.display().to_string()) isDirectory: false];
                        let drag_item: id = msg_send![class!(NSDraggingItem), alloc];
                        let item: id = msg_send![drag_item, initWithPasteboardWriter: nsurl];
                        let _: () = msg_send![item, autorelease];

                        let _: () = msg_send![item, setDraggingFrame: image_rect contents: img];

                        let _: () = msg_send![file_items, addObject: item];
                    }
                }
            }

            let drag_event: id = msg_send![class!(NSEvent), alloc];
            let current_event: id = msg_send![NSApp(), currentEvent];
            let drag_event: id = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure_(
        drag_event,
        NSEventType::NSLeftMouseDragged,
        current_position,
      NSEventModifierFlags::empty(),
        msg_send![current_event, timestamp],
        msg_send![window, windowNumber],
        nil,
         0,
          1,
          1.0
        );

            let cls = ClassDecl::new("DragRsSource", class!(NSObject));
            let cls = match cls {
                Some(mut cls) => {
                    cls.add_method(
                        sel!(draggingSession:sourceOperationMaskForDraggingContext:),
                        dragging_session
                            as extern "C" fn(&Object, Sel, id, NSUInteger) -> NSUInteger,
                    );

                    extern "C" fn dragging_session(
                        _this: &Object,
                        _: Sel,
                        _dragging_session: id,
                        context: NSUInteger,
                    ) -> NSUInteger {
                        if context == 0 {
                            // NSDragOperationCopy
                            1
                        } else {
                            // NSDragOperationEvery
                            NSUInteger::max_value()
                        }
                    }

                    cls.register()
                }
                None => Class::get("DragRsSource").expect("Failed to get the class definition"),
            };

            let source: id = msg_send![cls, alloc];
            let source: id = msg_send![source, init];

            let _: () = msg_send![ns_view, beginDraggingSessionWithItems: file_items event: drag_event source: source];
        }

        Ok(())
    } else {
        Err(crate::Error::UnsupportedWindowHandle)
    }
}
