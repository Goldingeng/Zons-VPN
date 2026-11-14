use crate::ffi::*;
use crate::util::{fill, w};
use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

const WM_TRAY: u32 = WM_APP + 1;
const WM_WAKE: u32 = WM_APP + 2;
const CLASS: &str = "ClashTrayNative";

#[derive(Clone, Copy)]
pub enum Ev {
    Show,
    Sys,
    Tun,
    Quit,
}

pub struct Lock(HANDLE);

impl Drop for Lock {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

struct T {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    icons: [HICON; 3],
    taskbar: u32,
    handler: fn(Ev),
    state: fn() -> (bool, bool),
}

thread_local! {
    static TRAY: RefCell<Option<T>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut T) -> R) -> Option<R> {
    TRAY.with(|t| t.try_borrow_mut().ok()?.as_mut().map(f))
}

pub fn lock(wait: bool) -> Option<Lock> {
    let name = w("Local\\ClashTrayNative");
    for _ in 0..if wait { 100 } else { 1 } {
        let h = unsafe { CreateMutexW(null(), 1, name.as_ptr()) };
        if !h.is_null() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
            return Some(Lock(h));
        }
        unsafe {
            if !h.is_null() {
                CloseHandle(h);
            }
            Sleep(100);
        }
    }
    None
}

pub fn wake() {
    let h = unsafe { FindWindowW(w(CLASS).as_ptr(), null()) };
    if !h.is_null() {
        unsafe { PostMessageW(h, WM_WAKE, 0, 0) };
    }
}

fn icon(inst: HINSTANCE, id: usize, n: i32) -> HICON {
    unsafe { LoadImageW(inst, id as *const u16, IMAGE_ICON, n, n, 0) }
}

pub fn init(handler: fn(Ev), state: fn() -> (bool, bool)) {
    let inst = unsafe { GetModuleHandleW(null()) };
    let cls = w(CLASS);
    let wc = WNDCLASSW {
        style: 0,
        wndproc: Some(wndproc),
        cls_extra: 0,
        wnd_extra: 0,
        instance: inst,
        icon: null_mut(),
        cursor: null_mut(),
        background: null_mut(),
        menu_name: null(),
        class_name: cls.as_ptr(),
    };
    let hwnd = unsafe {
        RegisterClassW(&wc);
        CreateWindowExW(0, cls.as_ptr(), cls.as_ptr(), 0, 0, 0, 0, 0, null_mut(), null_mut(), inst, null())
    };
    if hwnd.is_null() {
        return;
    }
    let n = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16);
    let icons = [icon(inst, 2, n), icon(inst, 1, n), icon(inst, 3, n)];
    let mut nid: NOTIFYICONDATAW = unsafe { zeroed() };
    nid.size = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hwnd = hwnd;
    nid.id = 1;
    nid.flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.callback = WM_TRAY;
    nid.icon = icons[0];
    fill(&mut nid.tip, "Zons VPN");
    unsafe { Shell_NotifyIconW(NIM_ADD, &nid) };
    let taskbar = unsafe { RegisterWindowMessageW(w("TaskbarCreated").as_ptr()) };
    TRAY.with(|t| *t.borrow_mut() = Some(T { hwnd, nid, icons, taskbar, handler, state }));
}

pub fn set(idx: usize, tip: &str) {
    with(|t| {
        t.nid.icon = t.icons[idx.min(2)];
        fill(&mut t.nid.tip, tip);
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &t.nid) };
    });
}

pub fn notify(title: &str, text: &str, err: bool) {
    with(|t| {
        t.nid.flags |= NIF_INFO;
        fill(&mut t.nid.info_title, title);
        fill(&mut t.nid.info, text);
        t.nid.info_flags = if err { NIIF_ERROR } else { NIIF_INFO };
        unsafe { Shell_NotifyIconW(NIM_MODIFY, &t.nid) };
        t.nid.flags &= !NIF_INFO;
    });
}

pub fn remove() {
    if let Some(t) = TRAY.with(|t| t.borrow_mut().take()) {
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &t.nid);
            for i in t.icons {
                DestroyIcon(i);
            }
            DestroyWindow(t.hwnd);
        }
    }
}

fn add(m: HMENU, id: usize, text: &str, checked: bool) {
    unsafe { AppendMenuW(m, MF_STRING | if checked { MF_CHECKED } else { 0 }, id, w(text).as_ptr()) };
}

fn menu(h: HWND) {
    let Some((handler, state)) = with(|t| (t.handler, t.state)) else { return };
    let (sys, tun) = state();
    let id = unsafe {
        let m = CreatePopupMenu();
        add(m, 1, "Открыть", false);
        AppendMenuW(m, MF_SEPARATOR, 0, null());
        add(m, 2, "Системный прокси", sys);
        add(m, 3, "TUN", tun);
        AppendMenuW(m, MF_SEPARATOR, 0, null());
        add(m, 4, "Выход", false);
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(h);
        let id = TrackPopupMenu(m, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, pt.x, pt.y, 0, h, null());
        PostMessageW(h, WM_NULL, 0, 0);
        DestroyMenu(m);
        id
    };
    let ev = match id {
        1 => Ev::Show,
        2 => Ev::Sys,
        3 => Ev::Tun,
        4 => Ev::Quit,
        _ => return,
    };
    handler(ev);
}

unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: usize, lp: isize) -> isize {
    match m {
        WM_TRAY => {
            match lp as u32 {
                WM_LBUTTONUP => {
                    if let Some(f) = with(|t| t.handler) {
                        f(Ev::Show);
                    }
                }
                WM_RBUTTONUP => menu(h),
                _ => {}
            }
            0
        }
        WM_WAKE => {
            if let Some(f) = with(|t| t.handler) {
                f(Ev::Show);
            }
            0
        }
        _ => {
            if m >= 0xC000 && with(|t| t.taskbar == m).unwrap_or(false) {
                with(|t| Shell_NotifyIconW(NIM_ADD, &t.nid));
            }
            DefWindowProcW(h, m, wp, lp)
        }
    }
}
