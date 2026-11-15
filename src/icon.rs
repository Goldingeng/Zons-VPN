use crate::ffi::*;
use crate::util::w;
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

pub type Px = SharedPixelBuffer<Rgba8Pixel>;

pub fn load(path: &str, size: u32) -> Option<Px> {
    extract(path, 0, size).or_else(|| {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        extract(&format!("{root}\\System32\\shell32.dll"), 2, size)
    })
}

fn extract(path: &str, index: i32, size: u32) -> Option<Px> {
    let mut h = null_mut();
    let r = unsafe { SHDefExtractIconW(w(path).as_ptr(), index, 0, &mut h, null_mut(), size) };
    if r != 0 || h.is_null() {
        return None;
    }
    let px = pixels(h);
    unsafe { DestroyIcon(h) };
    px
}

fn pixels(h: HICON) -> Option<Px> {
    let mut info: ICONINFO = unsafe { zeroed() };
    if unsafe { GetIconInfo(h, &mut info) } == 0 {
        return None;
    }
    let dc = unsafe { CreateCompatibleDC(null_mut()) };
    let out = read(dc, &info);
    unsafe {
        DeleteDC(dc);
        DeleteObject(info.color);
        DeleteObject(info.mask);
    }
    out
}

fn bits(dc: HANDLE, bmp: HANDLE) -> Option<(u32, u32, Vec<u8>)> {
    let mut hd: BITMAPINFOHEADER = unsafe { zeroed() };
    hd.size = size_of::<BITMAPINFOHEADER>() as u32;
    if unsafe { GetDIBits(dc, bmp, 0, 0, null_mut(), &mut hd, 0) } == 0 || hd.width <= 0 || hd.height == 0 {
        return None;
    }
    let (wd, ht) = (hd.width as u32, hd.height.unsigned_abs());
    hd.height = -(ht as i32);
    hd.bits = 32;
    hd.planes = 1;
    hd.compression = 0;
    let mut buf = vec![0u8; (wd * ht * 4) as usize];
    if unsafe { GetDIBits(dc, bmp, 0, ht, buf.as_mut_ptr().cast(), &mut hd, 0) } == 0 {
        return None;
    }
    Some((wd, ht, buf))
}

fn read(dc: HANDLE, info: &ICONINFO) -> Option<Px> {
    if info.color.is_null() {
        return None;
    }
    let (wd, ht, mut buf) = bits(dc, info.color)?;
    if buf.chunks(4).all(|p| p[3] == 0) {
        let (_, _, mask) = bits(dc, info.mask)?;
        for (p, m) in buf.chunks_mut(4).zip(mask.chunks(4)) {
            p[3] = if m[0] == 0 { 255 } else { 0 };
        }
    }
    let mut px = Px::new(wd, ht);
    for (d, s) in px.make_mut_slice().iter_mut().zip(buf.chunks(4)) {
        *d = Rgba8Pixel { r: s[2], g: s[1], b: s[0], a: s[3] };
    }
    Some(px)
}
