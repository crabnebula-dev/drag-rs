use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use crate::{DragItem, Image};

use std::{
    ffi::{c_void, OsStr},
    iter::once,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
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
        System::Ole::OleInitialize,
        System::Ole::{IDropSource, IDropSource_Impl, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY},
        System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        UI::{
            Shell::{
                CLSID_DragDropHelper, IDragSourceHelper, SHCreateDataObject, SHDoDragDrop,
                DROPFILES, SHDRAGIMAGE,
            },
            WindowsAndMessaging::{LoadImageW, IMAGE_BITMAP, LR_DEFAULTSIZE, LR_LOADFROMFILE},
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
struct DataObject {
    files: Vec<PathBuf>,
    inner_shell_obj: IDataObject,
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
    fn new(files: Vec<PathBuf>) -> Self {
        unsafe {
            Self {
                files,
                inner_shell_obj: SHCreateDataObject(None, None, None).unwrap(),
            }
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
        unsafe {
            if Self::is_supported_format(pformatetc) {
                let mut buffer = Vec::new();
                for path in &self.files {
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
                Ok(STGMEDIUM {
                    tymed: TYMED_HGLOBAL.0 as u32,
                    u: STGMEDIUM_0 { hGlobal: handle },
                    pUnkForRelease: std::mem::ManuallyDrop::new(None),
                })
            } else {
                self.inner_shell_obj.GetData(pformatetc)
            }
        }
    }

    fn GetDataHere(&self, _pformatetc: *const FORMATETC, _pmedium: *mut STGMEDIUM) -> Result<()> {
        Err(Error::new(DV_E_FORMATETC, HSTRING::new()))
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        unsafe {
            if Self::is_supported_format(pformatetc) {
                S_OK
            } else {
                self.inner_shell_obj.QueryGetData(pformatetc)
            }
        }
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
        frelease: BOOL,
    ) -> Result<()> {
        unsafe { self.inner_shell_obj.SetData(pformatetc, pmedium, frelease) }
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
        drop(&self.inner_shell_obj);
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
                let data_object: IDataObject = DataObject::new(files).into();
                let drop_source: IDropSource = DropSource::new().into();
                let helper: IDragSourceHelper = create_instance(&CLSID_DragDropHelper).unwrap();
                let hbitmap = match image {
                    Image::Raw(bytes) => create_dragimage_bitmap(bytes),
                    Image::File(path) => unsafe {
                        let wide_path: Vec<u16> =
                            path.as_os_str().encode_wide().chain(once(0)).collect();

                        match LoadImageW(
                            HMODULE::default(),
                            PCWSTR::from_raw(wide_path.as_ptr()),
                            IMAGE_BITMAP,
                            128_i32,
                            128_i32,
                            LR_DEFAULTSIZE | LR_LOADFROMFILE,
                        ) {
                            Ok(handle) => HBITMAP(handle.0),
                            Err(_) => HBITMAP(0),
                        }
                    },
                };

                unsafe {
                    let image = SHDRAGIMAGE {
                        sizeDragImage: SIZE { cx: 128, cy: 128 },
                        ptOffset: POINT { x: 0, y: 0 },
                        hbmpDragImage: hbitmap,
                        crColorKey: COLORREF(0xFFFFFFFF),
                    };

                    match helper.InitializeFromBitmap(&image, &data_object) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(e.into());
                        }
                    }

                    let _ = SHDoDragDrop(
                        HWND(_w.hwnd as isize),
                        &data_object,
                        &drop_source,
                        DROPEFFECT_COPY,
                    );
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

pub fn create_dragimage_bitmap(image: Vec<u8>) -> HBITMAP {
    let width = 512;
    let height = 512;
    let bitmap = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0_u32,
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
            let src_line = image.as_ptr().offset(y * 256_isize);

            let dst_line = ptr.offset(y * dst_stride);

            for x in (0..dst_stride).step_by(4) {
                let (r, g, b, a) = (
                    *src_line.offset(x) as i32,
                    *src_line.offset(x + 1) as i32,
                    *src_line.offset(x + 2) as i32,
                    *src_line.offset(x + 3) as i32,
                );

                let (r, g, b) = if a == 0 {
                    (0, 0, 0)
                } else {
                    (r * 255 / a, g * 255 / a, b * 255 / a)
                };
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
