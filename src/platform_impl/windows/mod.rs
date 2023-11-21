use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use crate::{DragItem, Image};

use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{c_void, OsStr},
    mem::size_of,
    os::windows::ffi::OsStrExt,
    sync::Once,
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::{
            CreateDIBSection, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
            HBITMAP,
        },
        System::Com::*,
        System::Memory::*,
        System::Ole::{IDropSource, IDropSource_Impl, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY},
        System::Ole::{OleInitialize, ReleaseStgMedium},
        System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        UI::{
            Shell::{
                CLSID_DragDropHelper, IDragSourceHelper, SHDoDragDrop, DROPFILES, SHDRAGIMAGE,
            },
            WindowsAndMessaging::GetCursorPos,
        },
    },
};

static mut OLE_RESULT: Result<()> = Ok(());
static OLE_UNINITIALIZE: Once = Once::new();
fn init_ole() {
    OLE_UNINITIALIZE.call_once(|| {
        unsafe {
            OLE_RESULT = OleInitialize(Some(std::ptr::null_mut()));
        }
        // I guess we never deinitialize for now?
        // OleUninitialize
    });
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct FormatetcKey {
    cf_format: u16,
    tymed: u32,
}

#[implement(IDataObject)]
#[derive(Clone)]
struct DataObject {
    h_global: HGLOBAL,
    stgms: RefCell<HashMap<FormatetcKey, STGMEDIUM>>,
}

impl DataObject {
    fn set_stgmedium(&self, formatetc: &FORMATETC, stgmedium: STGMEDIUM) {
        let key = FormatetcKey {
            cf_format: formatetc.cfFormat,
            tymed: formatetc.tymed,
        };

        unsafe {
            let mut bm = self.stgms.borrow_mut();
            if let Some(stgm) = bm.get_mut(&key) {
                if stgm.tymed != TYMED_ISTREAM.0 as u32 && stgm.tymed != TYMED_ISTORAGE.0 as u32 {
                    ReleaseStgMedium(stgm);
                }
                bm.remove(&key);
            }

            bm.insert(key.clone(), stgmedium);
        }
    }
    fn get_stgmedium(&self, formatetc: &FORMATETC) -> Option<*const STGMEDIUM> {
        let key = FormatetcKey {
            cf_format: formatetc.cfFormat,
            tymed: formatetc.tymed,
        };
        self.stgms.borrow().get(&key).map(|v| v as *const _)
    }
}

#[implement(IDropSource)]
struct DropSource(());

impl DropSource {
    fn new() -> Self {
        Self(())
    }
}

#[allow(non_snake_case)]
impl IDropSource_Impl for DropSource {
    fn QueryContinueDrag(&self, fescapepressed: BOOL, grfkeystate: MODIFIERKEYS_FLAGS) -> HRESULT {
        if fescapepressed.as_bool() {
            DRAGDROP_S_CANCEL
        } else if (grfkeystate & MK_LBUTTON) == MODIFIERKEYS_FLAGS(0) {
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

#[implement()]
struct DummyRelease;

impl DataObject {
    fn new(handle: HGLOBAL) -> Self {
        Self {
            h_global: handle,
            stgms: RefCell::new(HashMap::new()),
        }
    }

    fn is_supported_format(pformatetc: *const FORMATETC) -> bool {
        if let Some(format_etc) = unsafe { pformatetc.as_ref() } {
            !(format_etc.tymed as i32 != TYMED_HGLOBAL.0
                || format_etc.cfFormat != CF_HDROP.0
                || format_etc.dwAspect != DVASPECT_CONTENT.0)
        } else {
            false
        }
    }
}

#[allow(non_snake_case)]
impl IDataObject_Impl for DataObject {
    fn GetData(&self, pformatetc: *const FORMATETC) -> Result<STGMEDIUM> {
        if let Some(format_etc) = unsafe { pformatetc.as_ref() } {
            if let Some(result) = self.get_stgmedium(format_etc) {
                let result = unsafe { result.read() };
                if result.tymed == TYMED_HGLOBAL.0 as u32 {
                    unsafe {
                        dbg!(result.u.hGlobal);
                    }
                } else {
                    dbg!(result.tymed);
                }
                // FIXME: it crash when tymed is ISTREAM
                Ok(result.clone())
            } else {
                Err(Error::new(DV_E_FORMATETC, HSTRING::new()))
            }
        } else if Self::is_supported_format(pformatetc) {
            Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 {
                    hGlobal: self.h_global,
                },
                pUnkForRelease: std::mem::ManuallyDrop::new(Some(DummyRelease.into())),
            })
        } else {
            Err(Error::new(DV_E_FORMATETC, HSTRING::new()))
        }
    }

    fn GetDataHere(&self, _pformatetc: *const FORMATETC, _pmedium: *mut STGMEDIUM) -> Result<()> {
        Err(Error::new(DV_E_FORMATETC, HSTRING::new()))
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        S_OK
        // if Self::is_supported_format(pformatetc) {
        //     S_OK
        // } else {
        //     DV_E_FORMATETC
        // }
    }

    fn GetCanonicalFormatEtc(
        &self,
        _pformatectin: *const FORMATETC,
        pformatetcout: *mut FORMATETC,
    ) -> HRESULT {
        unsafe { (*pformatetcout).ptd = std::ptr::null_mut() };
        E_NOTIMPL
    }

    fn SetData(
        &self,
        pformatetc: *const FORMATETC,
        pmedium: *const STGMEDIUM,
        _frelease: BOOL,
    ) -> Result<()> {
        unsafe {
            if let Some(format_etc) = pformatetc.as_ref() {
                self.set_stgmedium(format_etc, pmedium.read())
            }
            Ok(())
        }
    }

    fn EnumFormatEtc(&self, _dwdirection: u32) -> Result<IEnumFORMATETC> {
        Err(Error::new(E_NOTIMPL, HSTRING::new()))
    }

    fn DAdvise(
        &self,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: Option<&IAdviseSink>,
    ) -> Result<u32> {
        Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()))
    }

    fn DUnadvise(&self, _dwconnection: u32) -> Result<()> {
        Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()))
    }

    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()))
    }
}

impl Drop for DataObject {
    fn drop(&mut self) {
        let _ = unsafe { GlobalFree(self.h_global) };
    }
}

pub fn start_drag<W: HasRawWindowHandle>(
    handle: &W,
    item: DragItem,
    image: Image,
) -> crate::Result<()> {
    if let RawWindowHandle::Win32(_w) = handle.raw_window_handle() {
        match item {
            DragItem::Files(files) => {
                init_ole();
                unsafe {
                    if let Err(e) = &OLE_RESULT {
                        return Err(e.clone().into());
                    }
                }
                let mut buffer = Vec::new();
                for path in files {
                    let path = OsStr::new(&path);
                    for code in path.encode_wide() {
                        buffer.push(code);
                    }
                    buffer.push(0);
                }

                // We finish with a double null.
                buffer.push(0);

                let size = std::mem::size_of::<DROPFILES>() + buffer.len() * 2;
                let handle = get_hglobal(size, buffer)?;
                let data_object: IDataObject = DataObject::new(handle).into();
                let drop_source: IDropSource = DropSource::new().into();
                let helper: IDragSourceHelper = create_instance(&CLSID_DragDropHelper).unwrap();
                // let hbitmap = match image {
                //     Image::Raw(bytes) => create_dragimage_bitmap(bytes),
                //     _ => panic!("Not implemented"),
                // };

                unsafe {
                    let mut cursor_pos = POINT::default();
                    unsafe { GetCursorPos(&mut cursor_pos as *mut _) };

                    // let image = SHDRAGIMAGE {
                    //     sizeDragImage: SIZE { cx: 128, cy: 128 },
                    //     ptOffset: POINT { x: 0, y: 0 },
                    //     hbmpDragImage: hbitmap,
                    //     crColorKey: COLORREF(0xFFFFFFFF),
                    // };

                    let image = create_dummy_drag_icon();

                    match helper.InitializeFromBitmap(&image, &data_object) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(e.into());
                        }
                    }

                    let mut effect = DROPEFFECT(0);
                    let _ = unsafe {
                        SHDoDragDrop(
                            HWND(_w.hwnd as isize),
                            &data_object,
                            &drop_source,
                            DROPEFFECT_COPY,
                        )
                    };
                };
            }
        }

        Ok(())
    } else {
        Err(crate::Error::UnsupportedWindowHandle)
    }
}

fn get_hglobal(size: usize, buffer: Vec<u16>) -> Result<HGLOBAL> {
    let handle = unsafe { GlobalAlloc(GMEM_FIXED, size).unwrap() };
    let ptr = unsafe { GlobalLock(handle) };

    let header = ptr as *mut DROPFILES;
    unsafe {
        (*header).pFiles = std::mem::size_of::<DROPFILES>() as u32;
        (*header).fWide = BOOL(1);
        std::ptr::copy(
            buffer.as_ptr() as *const c_void,
            ptr.add(std::mem::size_of::<DROPFILES>()),
            buffer.len() * 2,
        );
        GlobalUnlock(handle)
    }?;
    Ok(handle)
}

pub fn create_instance<T: Interface + ComInterface>(clsid: &GUID) -> windows::core::Result<T> {
    unsafe { CoCreateInstance(clsid, None, CLSCTX_ALL) }
}

fn create_dummy_drag_icon() -> SHDRAGIMAGE {
    unsafe {
        let mut ptr = std::ptr::null_mut();
        let dc = GetDC(HWND(0));
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 128,
                biHeight: 128,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0_u32,
                biSizeImage: (128 * 128 * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: Default::default(),
        };

        let bitmap = CreateDIBSection(
            dc,
            &bitmap_info as *const _,
            DIB_RGB_COLORS,
            &mut ptr as *mut *mut _,
            HANDLE(0),
            0,
        )
        .unwrap();

        let dst_stride = (128 * 4) as isize;
        let ptr = ptr as *mut u8;
        for y in 0..128_isize {
            let dst_line = ptr.offset(y * dst_stride);

            for x in (0..dst_stride).step_by(4) {
                *dst_line.offset(x) = 0;
                *dst_line.offset(x + 1) = 0;
                *dst_line.offset(x + 2) = 255;
                *dst_line.offset(x + 3) = 255;
            }
        }

        ReleaseDC(HWND(0), dc);

        SHDRAGIMAGE {
            sizeDragImage: SIZE { cx: 128, cy: 128 },
            ptOffset: POINT { x: 64, y: 120 },
            hbmpDragImage: bitmap,
            crColorKey: COLORREF(0xFFFFFFFF),
        }
    }
}

pub fn create_dragimage_bitmap(image: Vec<u8>) -> HBITMAP {
    let width = 128;
    let height = 128;
    let bitmap = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0 as u32,
            biSizeImage: (width * height * 4) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: Default::default(),
    };

    unsafe {
        let dc = GetDC(HWND(0));

        let mut ptr = std::ptr::null_mut();

        let bitmap = CreateDIBSection(
            dc,
            &bitmap as *const _,
            DIB_RGB_COLORS,
            &mut ptr as *mut *mut _,
            HANDLE(0),
            0,
        );

        // Bitmap needs to be flipped and unpremultiplied

        let dst_stride = (width * 4) as isize;
        let ptr = ptr as *mut u8;
        for y in 0..height as isize {
            let src_line = image.as_ptr().offset(y * 256 as isize);

            let dst_line = ptr.offset(y * dst_stride);

            for x in (0..dst_stride).step_by(4) {
                let (r, g, b, a) = (
                    *src_line.offset(x) as i32,
                    *src_line.offset(x + 1) as i32,
                    *src_line.offset(x + 2) as i32,
                    *src_line.offset(x + 3) as i32,
                );

                // let (r, g, b) = if a == 0 {
                //     (0, 0, 0)
                // } else {
                //     (r * 255 / a, g * 255 / a, b * 255 / a)
                // };
                *dst_line.offset(x) = b as u8;
                *dst_line.offset(x + 1) = g as u8;
                *dst_line.offset(x + 2) = r as u8;
                *dst_line.offset(x + 3) = a as u8;
            }
        }

        ReleaseDC(HWND(0), dc);

        bitmap.unwrap()
    }
}
