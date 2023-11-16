use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use crate::{DragItem, Image};

use std::{
    ffi::{c_void, OsStr},
    os::windows::ffi::OsStrExt,
    sync::Once,
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::Com::*,
        System::Memory::*,
        System::Ole::OleInitialize,
        System::Ole::{
            DoDragDrop, IDropSource, IDropSource_Impl, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY,
        },
        System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        UI::Shell::DROPFILES,
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

#[implement(IDataObject)]
#[derive(Clone)]
struct DataObject(HGLOBAL);

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
        Self(handle)
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
        if Self::is_supported_format(pformatetc) {
            Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: self.0 },
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
        if Self::is_supported_format(pformatetc) {
            S_OK
        } else {
            DV_E_FORMATETC
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
        _pformatetc: *const FORMATETC,
        _pmedium: *const STGMEDIUM,
        _frelease: BOOL,
    ) -> Result<()> {
        Err(Error::new(E_NOTIMPL, HSTRING::new()))
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
        let _ = unsafe { GlobalFree(self.0) };
    }
}

pub fn start_drag<W: HasRawWindowHandle>(
    handle: &W,
    item: DragItem,
    _image: Image,
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

                let mut effect = DROPEFFECT(0);
                let _ =
                    unsafe { DoDragDrop(&data_object, &drop_source, DROPEFFECT_COPY, &mut effect) };
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
