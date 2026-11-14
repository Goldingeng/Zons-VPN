use crate::ffi::InternetSetOptionW;
use crate::util::w;
use std::mem::size_of;
use std::ptr::{null, null_mut};

const PER_CONNECTION: u32 = 75;
const SETTINGS_CHANGED: u32 = 95;
const REFRESH: u32 = 37;
const BYPASS: &str = "localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*;<local>";

#[repr(C)]
struct Opt {
    option: u32,
    value: u64,
}

#[repr(C)]
struct List {
    size: u32,
    conn: *const u16,
    count: u32,
    error: u32,
    opts: *mut Opt,
}

pub fn set(port: Option<u16>) -> bool {
    let server = w(&format!("127.0.0.1:{}", port.unwrap_or(0)));
    let bypass = w(BYPASS);
    let mut opts = [
        Opt { option: 1, value: if port.is_some() { 3 } else { 1 } },
        Opt { option: 2, value: server.as_ptr() as u64 },
        Opt { option: 3, value: bypass.as_ptr() as u64 },
    ];
    let mut l = List {
        size: size_of::<List>() as u32,
        conn: null(),
        count: if port.is_some() { 3 } else { 1 },
        error: 0,
        opts: opts.as_mut_ptr(),
    };
    unsafe {
        let r = InternetSetOptionW(null_mut(), PER_CONNECTION, &mut l as *mut List as _, l.size);
        InternetSetOptionW(null_mut(), SETTINGS_CHANGED, null_mut(), 0);
        InternetSetOptionW(null_mut(), REFRESH, null_mut(), 0);
        r != 0
    }
}
