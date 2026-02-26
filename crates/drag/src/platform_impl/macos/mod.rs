// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::ptr::NonNull;

use core_graphics::display::CGDisplay;
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_foundation::{NSArray, NSData, NSMutableArray, NSPoint, NSRect, NSSize, NSString, NSURL};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{CursorPosition, DragItem, DragMode, DragResult, Image, Options};
use objc2_app_kit::{
    NSApp, NSDraggingContext, NSDraggingItem, NSDraggingSession, NSDraggingSource, NSEvent,
    NSEventModifierFlags, NSEventType, NSImage, NSPasteboardItem, NSPasteboardItemDataProvider,
};

type OnDropCallback = Box<dyn Fn(DragResult, CursorPosition) + Send>;

#[derive(Clone, Copy)]
struct DataProviderPtr(NonNull<crate::DataProvider>);

impl DataProviderPtr {
    fn from_box(provider: crate::DataProvider) -> Self {
        let ptr = Box::into_raw(Box::new(provider));
        // SAFETY: Box::into_raw never returns null.
        Self(unsafe { NonNull::new_unchecked(ptr) })
    }

    unsafe fn as_ref(&self) -> &crate::DataProvider {
        // SAFETY: Pointer comes from Box::into_raw in from_box and remains valid
        // until consumed exactly once by into_box.
        unsafe { self.0.as_ref() }
    }

    unsafe fn into_box(self) -> Box<crate::DataProvider> {
        // SAFETY: Pointer originates from Box::into_raw and ownership is reclaimed once.
        unsafe { Box::from_raw(self.0.as_ptr()) }
    }
}

#[derive(Clone, Copy)]
struct OnDropCallbackPtr(NonNull<OnDropCallback>);

impl OnDropCallbackPtr {
    fn from_box(callback: OnDropCallback) -> Self {
        let ptr = Box::into_raw(Box::new(callback));
        // SAFETY: Box::into_raw never returns null.
        Self(unsafe { NonNull::new_unchecked(ptr) })
    }

    unsafe fn as_ref(&self) -> &OnDropCallback {
        // SAFETY: Pointer comes from Box::into_raw in from_box and remains valid
        // until consumed exactly once by into_box.
        unsafe { self.0.as_ref() }
    }

    unsafe fn into_box(self) -> Box<OnDropCallback> {
        // SAFETY: Pointer originates from Box::into_raw and ownership is reclaimed once.
        unsafe { Box::from_raw(self.0.as_ptr()) }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "DragRsDataProvider"]
    #[ivars = DragRsDataProviderIvars]
    struct DragRsDataProvider;

    unsafe impl NSObjectProtocol for DragRsDataProvider {}

    unsafe impl NSPasteboardItemDataProvider for DragRsDataProvider {
        #[unsafe(method(pasteboard:item:provideDataForType:))]
        unsafe fn provide_data(
            &self,
            _pasteboard: &objc2_app_kit::NSPasteboard,
            item: &NSPasteboardItem,
            data_type: &NSString,
        ) {
            let ivars = self.ivars();
            let provider = ivars.provider_ptr.as_ref();

            if let Some(data) = provider(&data_type.to_string()) {
                let ns_data = NSData::from_vec(data);
                let _: () = msg_send![item, setData: &*ns_data, forType: data_type];
            }
        }

        #[unsafe(method(pasteboardFinishedWithDataProvider:))]
        unsafe fn pasteboard_finished(&self, _pasteboard: &objc2_app_kit::NSPasteboard) {
            drop(self.ivars().provider_ptr.into_box());
        }
    }
);

struct DragRsDataProviderIvars {
    provider_ptr: DataProviderPtr,
}

impl DragRsDataProvider {
    pub fn new(provider: crate::DataProvider, mtm: MainThreadMarker) -> Retained<Self> {
        let provider_ptr = DataProviderPtr::from_box(provider);
        let this = Self::alloc(mtm).set_ivars(DragRsDataProviderIvars { provider_ptr });
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "DragRsSource"]
    #[ivars = DragRsSourceIvars]
    struct DragRsSource;

    unsafe impl NSObjectProtocol for DragRsSource {}

    unsafe impl NSDraggingSource for DragRsSource {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        unsafe fn dragging_session(
            &self,
            session: &NSDraggingSession,
            _context: NSDraggingContext,
        ) -> objc2_app_kit::NSDragOperation {
            let ivars = self.ivars();
            session
                .setAnimatesToStartingPositionsOnCancelOrFail(ivars.animate_on_cancel_or_failure);

            match ivars.drag_mode {
                DragMode::Copy => objc2_app_kit::NSDragOperation::Copy,
                DragMode::Move => objc2_app_kit::NSDragOperation::Move,
            }
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        unsafe fn dragging_session_end(
            &self,
            _session: &NSDraggingSession,
            ended_at_point: NSPoint,
            operation: objc2_app_kit::NSDragOperation,
        ) {
            let callback = self.ivars().on_drop_ptr;

            let mouse_location = CursorPosition {
                x: ended_at_point.x as i32,
                y: CGDisplay::main().pixels_high() as i32 - ended_at_point.y as i32,
            };

            let callback_closure = callback.as_ref();

            if operation == objc2_app_kit::NSDragOperation::None {
                callback_closure(DragResult::Cancel, mouse_location);
            } else {
                callback_closure(DragResult::Dropped, mouse_location);
            }

            drop(callback.into_box());
        }
    }
);

struct DragRsSourceIvars {
    on_drop_ptr: OnDropCallbackPtr,
    animate_on_cancel_or_failure: bool,
    drag_mode: DragMode,
}

impl DragRsSource {
    pub fn new<F: Fn(DragResult, CursorPosition) + Send + 'static>(
        on_drop_callback: F,
        options: &Options,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let on_drop_callback: OnDropCallback = Box::new(on_drop_callback);
        let callback_ptr = OnDropCallbackPtr::from_box(on_drop_callback);

        let this = Self::alloc(mtm).set_ivars(DragRsSourceIvars {
            on_drop_ptr: callback_ptr,
            animate_on_cancel_or_failure: !options.skip_animatation_on_cancel_or_failure,
            drag_mode: options.mode,
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub fn start_drag<W: HasWindowHandle, F: Fn(DragResult, CursorPosition) + Send + 'static>(
    handle: &W,
    item: DragItem,
    image: Image,
    on_drop_callback: F,
    options: Options,
) -> crate::Result<()> {
    if let Ok(RawWindowHandle::AppKit(w)) = handle.window_handle().map(|h| h.as_raw()) {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let ns_view: *mut objc2::runtime::AnyObject = w.ns_view.as_ptr() as *mut _;
            let window: Retained<objc2_app_kit::NSWindow> = msg_send![ns_view, window];
            let ns_view = window.contentView().expect("Failed to get contentView");

            let current_position: NSPoint = window.mouseLocationOutsideOfEventStream();

            let img = match image {
                Image::File(path) => {
                    if !path.exists() {
                        return Err(crate::Error::ImageNotFound);
                    }
                    NSImage::initByReferencingFile(
                        NSImage::alloc(),
                        &NSString::from_str(&path.to_string_lossy()),
                    )
                }
                Image::Raw(bytes) => {
                    let data = NSData::from_vec(bytes);
                    NSImage::initWithData(NSImage::alloc(), &data)
                }
            };
            let img = img.expect("Failed to create NSImage");
            let image_size: NSSize = img.size();
            let image_rect = NSRect::new(
                NSPoint::new(
                    current_position.x - image_size.width / 2.,
                    current_position.y - image_size.height / 2.,
                ),
                image_size,
            );

            let dragging_items = NSMutableArray::new();

            match item {
                DragItem::Files(files) => {
                    for path in files {
                        let nsurl = NSURL::fileURLWithPath_isDirectory(
                            &NSString::from_str(&path.display().to_string()),
                            false,
                        );
                        let item = NSDraggingItem::initWithPasteboardWriter(
                            NSDraggingItem::alloc(),
                            &ProtocolObject::from_retained(nsurl),
                        );
                        item.setDraggingFrame_contents(image_rect, Some(&*img));
                        dragging_items.addObject(&*item);
                    }
                }
                DragItem::Data { provider, types } => {
                    let data_provider = DragRsDataProvider::new(provider, mtm);

                    let item = NSPasteboardItem::new();
                    let types_array = types
                        .into_iter()
                        .map(|t| NSString::from_str(&t))
                        .collect::<Vec<_>>();
                    let types_array = NSArray::from_retained_slice(&types_array);

                    item.setDataProvider_forTypes(
                        &ProtocolObject::from_retained(data_provider),
                        &types_array,
                    );

                    let drag_item = NSDraggingItem::initWithPasteboardWriter(
                        NSDraggingItem::alloc(),
                        &ProtocolObject::from_retained(item),
                    );
                    drag_item.setDraggingFrame_contents(image_rect, Some(&*img));
                    dragging_items.addObject(&*drag_item);
                }
            }

            let current_event = NSApp(mtm).currentEvent();
            let timestamp = current_event.map(|e| e.timestamp()).unwrap_or(0.0);
            let window_number = window.windowNumber();

            let drag_event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                NSEventType::LeftMouseDragged,
                current_position,
                NSEventModifierFlags::empty(),
                timestamp,
                window_number,
                None,
                0,
                1,
                1.0
            ).expect("Failed to create NSEvent");

            let source = DragRsSource::new(on_drop_callback, &options, mtm);

            let _: Retained<NSDraggingSession> = msg_send![
                &*ns_view,
                beginDraggingSessionWithItems: &*dragging_items,
                event: &*drag_event,
                source: &*ProtocolObject::<dyn NSDraggingSource>::from_retained(source),
            ];

            Ok(())
        }
    } else {
        Err(crate::Error::UnsupportedWindowHandle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    };

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn macos_objc2_data_provider_roundtrip_and_cleanup() {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let calls = Arc::new(AtomicUsize::new(0));
            let dropped = Arc::new(AtomicBool::new(false));
            let dropped_guard = DropFlag(dropped.clone());
            let expected = b"drag-rs-objc2".to_vec();

            let provider = {
                let calls = calls.clone();
                let expected = expected.clone();
                Box::new(move |data_type: &str| {
                    let _drop_guard = &dropped_guard;
                    calls.fetch_add(1, Ordering::SeqCst);
                    if data_type == "public.utf8-plain-text" {
                        Some(expected.clone())
                    } else {
                        None
                    }
                }) as crate::DataProvider
            };

            let data_provider = DragRsDataProvider::new(provider, mtm);
            let item = NSPasteboardItem::new();
            let data_type = NSString::from_str("public.utf8-plain-text");

            let _: () = msg_send![
                &*data_provider,
                pasteboard: std::ptr::null::<objc2_app_kit::NSPasteboard>(),
                item: &*item,
                provideDataForType: &*data_type
            ];

            let roundtrip = item
                .dataForType(&data_type)
                .expect("provider should set NSData for requested type");
            assert_eq!(roundtrip.to_vec(), expected);
            assert_eq!(calls.load(Ordering::SeqCst), 1);

            let _: () = msg_send![
                &*data_provider,
                pasteboardFinishedWithDataProvider: std::ptr::null::<objc2_app_kit::NSPasteboard>()
            ];
            assert!(dropped.load(Ordering::SeqCst));
        }
    }

    #[test]
    fn macos_objc2_drag_source_callback_path() {
        unsafe {
            let mtm = MainThreadMarker::new_unchecked();
            let dropped = Arc::new(AtomicBool::new(false));
            let dropped_guard = DropFlag(dropped.clone());
            let observed = Arc::new(Mutex::new(None::<DragResult>));
            let observed_clone = observed.clone();

            let source = DragRsSource::new(
                move |result, _cursor| {
                    let _drop_guard = &dropped_guard;
                    *observed_clone.lock().expect("poisoned mutex") = Some(result);
                },
                &Options::default(),
                mtm,
            );

            let _: () = msg_send![
                &*source,
                draggingSession: std::ptr::null::<objc2_app_kit::NSDraggingSession>(),
                endedAtPoint: NSPoint::new(10.0, 20.0),
                operation: objc2_app_kit::NSDragOperation::None
            ];

            let result = *observed.lock().expect("poisoned mutex");
            assert!(matches!(result, Some(DragResult::Cancel)));
            assert!(dropped.load(Ordering::SeqCst));
        }
    }
}
