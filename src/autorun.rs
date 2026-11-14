use crate::ffi::*;
use crate::util::{exe, w};
use std::ptr::null_mut;

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const NAME: &str = "ClashTray";

fn key(sam: u32) -> Option<HKEY> {
    let mut k = null_mut();
    (unsafe { RegOpenKeyExW(hkcu(), w(RUN).as_ptr(), 0, sam, &mut k) } == 0).then_some(k)
}

pub fn get() -> bool {
    let Some(k) = key(KEY_QUERY_VALUE) else { return false };
    let r = unsafe { RegQueryValueExW(k, w(NAME).as_ptr(), null_mut(), null_mut(), null_mut(), null_mut()) };
    unsafe { RegCloseKey(k) };
    r == 0
}

pub fn set(on: bool) {
    let Some(k) = key(KEY_SET_VALUE) else { return };
    unsafe {
        if on {
            let v = w(&format!("\"{}\" --hidden", exe().display()));
            RegSetValueExW(k, w(NAME).as_ptr(), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32);
        } else {
            RegDeleteValueW(k, w(NAME).as_ptr());
        }
        RegCloseKey(k);
    }
}
