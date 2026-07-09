use crate::win::*;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::OnceLock;

static COM_INITED: OnceLock<Result<(), String>> = OnceLock::new();
static VDM_PTR: OnceLock<usize> = OnceLock::new();

fn vdm() -> Option<*mut c_void> {
    VDM_PTR.get().map(|&p| p as *mut c_void)
}

pub fn initialize() -> Result<(), String> {
    COM_INITED.get_or_init(|| unsafe {
        let hr = CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED);
        if hr != S_OK {
            return Err(format!("COM initialization failed: HRESULT {hr}"));
        }
        let mut mgr: *mut c_void = null_mut();
        let hr = CoCreateInstance(
            &CLSID_VirtualDesktopManager as *const GUID,
            null_mut(),
            CLSCTX_LOCAL_SERVER,
            &IID_IVirtualDesktopManager as *const GUID,
            &mut mgr,
        );
        if hr != S_OK || mgr.is_null() {
            return Err("VirtualDesktopManager not available".to_string());
        }
        VDM_PTR.set(mgr as usize).ok();
        Ok(())
    }).clone()
}

pub fn current_desktop_id() -> Option<String> {
    let init = initialize();
    if init.is_err() {
        return None;
    }
    let mgr = vdm()?;
    let mut guid = GUID::default();
    unsafe {
        let vtbl = &*(*(mgr as *mut *mut VirtualDesktopManagerVtbl));
        let hr = (vtbl.get_window_desktop_id)(mgr, null_mut(), &mut guid);
        if hr != S_OK {
            return None;
        }
    }
    let id = format!(
        "{:08X}-{:04X}-{:04X}",
        guid.Data1, guid.Data2, guid.Data3,
    );
    Some(id)
}

pub fn desktop_label() -> String {
    match current_desktop_id() {
        None => String::new(),
        Some(id) => {
            let env_key = format!("PARKER_DESKTOP_{}", id.replace('-', "_"));
            std::env::var(&env_key).unwrap_or(id)
        }
    }
}
