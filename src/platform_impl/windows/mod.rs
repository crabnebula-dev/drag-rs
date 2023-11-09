use raw_window_handle::HasRawWindowHandle;

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
        System::Ole::{
            DoDragDrop, IDropSource, IDropSource_Impl, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY,
        },
        System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        UI::Shell::DROPFILES,
    },
};

static OLE_UNINITIALIZE: Once = Once::new();
fn init_ole() {
    let _ = OLE_UNINITIALIZE.call_once(|| {
        use windows::Win32::System::Ole::OleInitialize;
        let _ = unsafe { dbg!(OleInitialize(Some(std::ptr::null_mut()))) }.unwrap();

        // I guess we never deinitialize for now?
        // OleUninitialize
    });
}

#[implement(IDataObject)]
struct DataObject(HGLOBAL);

#[implement(IDropSource)]
struct DropSource(());

impl DropSource {
    fn new() -> Self {
        return Self(());
    }
}

impl DataObject {
    fn new(handle: HGLOBAL) -> Self {
        return Self(handle);
    }

    fn is_supported_format(pformatetc: *const FORMATETC) -> bool {
        if let Some(format_etc) = unsafe { pformatetc.as_ref() } {
            dbg!(format_etc);
            if format_etc.tymed as i32 != TYMED_HGLOBAL.0 {
                return false;
            }
            if format_etc.cfFormat != CF_HDROP.0 {
                return false;
            }
            if format_etc.dwAspect != DVASPECT_CONTENT.0 {
                return false;
            }
            return true;
        } else {
            return false;
        }
    }
}

#[allow(non_snake_case)]
impl IDataObject_Impl for DataObject {
    fn GetData(&self, pformatetc: *const FORMATETC) -> Result<STGMEDIUM> {
        if Self::is_supported_format(pformatetc) {
            return Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: self.0 },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            });
        } else {
            return Err(Error::new(DV_E_FORMATETC, HSTRING::new()));
        }
    }

    fn GetDataHere(&self, _pformatetc: *const FORMATETC, _pmedium: *mut STGMEDIUM) -> Result<()> {
        return Err(Error::new(DV_E_FORMATETC, HSTRING::new()));
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        if Self::is_supported_format(pformatetc) {
            return S_OK;
        } else {
            return DV_E_FORMATETC;
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        _pformatectin: *const FORMATETC,
        pformatetcout: *mut FORMATETC,
    ) -> HRESULT {
        unsafe { (*pformatetcout).ptd = std::ptr::null_mut() };
        return E_NOTIMPL;
    }

    fn SetData(
        &self,
        _pformatetc: *const FORMATETC,
        _pmedium: *const STGMEDIUM,
        _frelease: BOOL,
    ) -> Result<()> {
        return Err(Error::new(E_NOTIMPL, HSTRING::new()));
    }

    fn EnumFormatEtc(&self, _dwdirection: u32) -> Result<IEnumFORMATETC> {
        return Err(Error::new(E_NOTIMPL, HSTRING::new()));
    }

    fn DAdvise(
        &self,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: Option<&IAdviseSink>,
    ) -> Result<u32> {
        return Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()));
    }

    fn DUnadvise(&self, _dwconnection: u32) -> Result<()> {
        return Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()));
    }

    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        return Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()));
    }
}

impl IDropSource_Impl for DropSource {
    fn QueryContinueDrag(&self, fescapepressed: BOOL, grfkeystate: MODIFIERKEYS_FLAGS) -> HRESULT {
        if fescapepressed.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        if (grfkeystate & MK_LBUTTON) == MODIFIERKEYS_FLAGS(0) {
            return DRAGDROP_S_DROP;
        }
        return S_OK;
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        return DRAGDROP_S_USEDEFAULTCURSORS;
    }
}

pub fn start_drag<W: HasRawWindowHandle>(handle: &W, item: DragItem, image: Image) {
    match item {
        DragItem::Files(files) => {
            init_ole();
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
            let handle = unsafe { GlobalAlloc(GMEM_FIXED, size).unwrap() };
            let ptr = unsafe { GlobalLock(handle) };

            let header = ptr as *mut DROPFILES;
            unsafe {
                (*header).pFiles = std::mem::size_of::<DROPFILES>() as u32;
                (*header).fWide = BOOL(1);
            }

            unsafe {
                std::ptr::copy(
                    buffer.as_ptr() as *const c_void,
                    ptr.add(std::mem::size_of::<DROPFILES>()),
                    buffer.len() * 2,
                )
            };
            unsafe { GlobalUnlock(handle) };

            let data_object : IDataObject = DataObject::new(handle).into();
            let drop_source : IDropSource = DropSource::new().into();

            let mut effect = DROPEFFECT(0);
            let _ = unsafe {
                dbg!(DoDragDrop(
                    &data_object,
                    &drop_source,
                    DROPEFFECT_COPY,
                    &mut effect
                ))
            };
        }
    }
}
