use crate::ffi::*;
use crate::util::w;
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::mem::{size_of, zeroed};
use std::ffi::c_void;
use std::ptr::null_mut;

pub type Px = SharedPixelBuffer<Rgba8Pixel>;

pub fn load(path: &str, size: u32) -> Option<Px> {
    let big = (size * 2).clamp(48, 256);
    let src = extract(path, 0, big).or_else(|| {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        extract(&format!(r"{root}\System32\shell32.dll"), 2, big)
    })?;
    Some(shrink(&src, size))
}

fn shrink(src: &Px, size: u32) -> Px {
    let (sw, sh) = (src.width(), src.height());
    if sw <= size || sh <= size {
        return src.clone();
    }
    let data = src.as_slice();
    let mut out = Px::new(size, size);
    for (i, d) in out.make_mut_slice().iter_mut().enumerate() {
        let (x, y) = (i as u32 % size, i as u32 / size);
        let (x0, x1) = (x * sw / size, ((x + 1) * sw / size).max(x * sw / size + 1));
        let (y0, y1) = (y * sh / size, ((y + 1) * sh / size).max(y * sh / size + 1));
        let (mut r, mut g, mut b, mut a, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let p = data[(yy * sw + xx) as usize];
                let pa = p.a as u32;
                r += p.r as u32 * pa;
                g += p.g as u32 * pa;
                b += p.b as u32 * pa;
                a += pa;
                n += 1;
            }
        }
        *d = if a == 0 {
            Rgba8Pixel { r: 0, g: 0, b: 0, a: 0 }
        } else {
            Rgba8Pixel { r: (r / a) as u8, g: (g / a) as u8, b: (b / a) as u8, a: (a / n) as u8 }
        };
    }
    out
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

pub fn title(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let file = w(path);
    let size = unsafe { GetFileVersionInfoSizeW(file.as_ptr(), null_mut()) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0u8; size as usize];
    if unsafe { GetFileVersionInfoW(file.as_ptr(), 0, size, data.as_mut_ptr().cast()) } == 0 {
        return None;
    }
    let langs = query(&data, "\\VarFileInfo\\Translation");
    let mut ids: Vec<String> = langs.chunks_exact(4).map(|c| format!("{:04x}{:04x}", u16::from_le_bytes([c[0], c[1]]), u16::from_le_bytes([c[2], c[3]]))).collect();
    ids.extend(["040904b0".into(), "040904e4".into(), "000004b0".into()]);
    for key in ["FileDescription", "ProductName"] {
        for id in &ids {
            let raw = query(&data, &format!("\\StringFileInfo\\{id}\\{key}"));
            let u: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
            let t = String::from_utf16_lossy(&u).trim().to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    None
}

fn query(data: &[u8], sub: &str) -> Vec<u8> {
    let mut p: *mut c_void = null_mut();
    let mut len = 0u32;
    if unsafe { VerQueryValueW(data.as_ptr().cast(), w(sub).as_ptr(), &mut p, &mut len) } == 0 || p.is_null() {
        return Vec::new();
    }
    let n = if sub.contains("Translation") { len as usize } else { len as usize * 2 };
    unsafe { std::slice::from_raw_parts(p as *const u8, n).to_vec() }
}
