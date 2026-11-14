use crate::ffi::*;
use crate::util::w;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

const NO_PROXY: u32 = 1;
const NAMED_PROXY: u32 = 3;
const FLAG_SECURE: u32 = 0x0080_0000;
const OPT_DECOMPRESSION: u32 = 118;
const QUERY_STATUS: u32 = 19 | 0x2000_0000;

struct H(*mut c_void);

impl Drop for H {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

fn split(url: &str) -> Option<(bool, String, u16, String)> {
    let url = url.trim();
    let (tls, rest) = match url.strip_prefix("https://") {
        Some(r) => (true, r),
        None => (false, url.strip_prefix("http://")?),
    };
    let (auth, path) = match rest.find(['/', '?']) {
        Some(i) => (&rest[..i], rest[i..].to_string()),
        None => (rest, "/".to_string()),
    };
    let path = if path.starts_with('?') { format!("/{path}") } else { path };
    let auth = auth.rsplit('@').next()?;
    let (host, port) = match auth.strip_prefix('[') {
        Some(r) => {
            let e = r.find(']')?;
            (r[..e].to_string(), r[e + 1..].strip_prefix(':').and_then(|p| p.parse().ok()))
        }
        None => match auth.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().ok()),
            None => (auth.to_string(), None),
        },
    };
    Some((tls, host, port.unwrap_or(if tls { 443 } else { 80 }), path))
}

fn fail(s: &str) -> String {
    format!("{s}: {}", std::io::Error::last_os_error())
}

pub fn get(url: &str, ua: &str, proxy: Option<u16>) -> Result<Vec<u8>, String> {
    let (tls, host, port, path) = split(url).ok_or("некорректный URL")?;
    let px = proxy.map(|p| w(&format!("127.0.0.1:{p}")));
    unsafe {
        let s = H(WinHttpOpen(w(ua).as_ptr(), if px.is_some() { NAMED_PROXY } else { NO_PROXY }, px.as_ref().map_or(null(), |p| p.as_ptr()), null(), 0));
        if s.0.is_null() {
            return Err(fail("WinHttpOpen"));
        }
        WinHttpSetTimeouts(s.0, 10_000, 10_000, 15_000, 30_000);
        let all: u32 = 3;
        WinHttpSetOption(s.0, OPT_DECOMPRESSION, &all as *const u32 as _, 4);
        let c = H(WinHttpConnect(s.0, w(&host).as_ptr(), port, 0));
        if c.0.is_null() {
            return Err(fail("подключение"));
        }
        let r = H(WinHttpOpenRequest(c.0, w("GET").as_ptr(), w(&path).as_ptr(), null(), null(), null(), if tls { FLAG_SECURE } else { 0 }));
        if r.0.is_null() {
            return Err(fail("запрос"));
        }
        if WinHttpSendRequest(r.0, null(), 0, null(), 0, 0, 0) == 0 || WinHttpReceiveResponse(r.0, null_mut()) == 0 {
            return Err(fail("запрос"));
        }
        let mut code = 0u32;
        let mut len = 4u32;
        WinHttpQueryHeaders(r.0, QUERY_STATUS, null(), &mut code as *mut u32 as _, &mut len, null_mut());
        if code != 200 {
            return Err(format!("HTTP {code}"));
        }
        let mut out = Vec::new();
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let mut n = 0u32;
            if WinHttpReadData(r.0, buf.as_mut_ptr() as _, buf.len() as u32, &mut n) == 0 {
                return Err(fail("чтение"));
            }
            if n == 0 {
                return Ok(out);
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
    }
}
