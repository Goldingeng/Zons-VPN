use crate::ffi::*;
use crate::util::w;
use std::mem::{size_of, zeroed};

pub fn pick_exe() -> Option<String> {
    let filter: Vec<u16> = "Программы (*.exe)\0*.exe\0\0".encode_utf16().collect();
    let title = w("Приложение, которое пойдёт мимо VPN");
    let mut buf = vec![0u16; 1024];
    let mut ofn: OPENFILENAMEW = unsafe { zeroed() };
    ofn.size = size_of::<OPENFILENAMEW>() as u32;
    ofn.filter = filter.as_ptr();
    ofn.file = buf.as_mut_ptr();
    ofn.max_file = buf.len() as u32;
    ofn.title = title.as_ptr();
    ofn.flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR;
    if unsafe { GetOpenFileNameW(&mut ofn) } == 0 {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let path = String::from_utf16_lossy(&buf[..n]);
    path.rsplit(['\\', '/']).next().map(str::to_string)
}
