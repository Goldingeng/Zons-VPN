#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type HWND = *mut c_void;
pub type HMENU = *mut c_void;
pub type HICON = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HKEY = *mut c_void;
pub type WNDPROC = unsafe extern "system" fn(HWND, u32, usize, isize) -> isize;

pub const WM_NULL: u32 = 0;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_APP: u32 = 0x8000;

pub const MF_STRING: u32 = 0x0;
pub const MF_CHECKED: u32 = 0x8;
pub const MF_SEPARATOR: u32 = 0x800;
pub const TPM_RIGHTBUTTON: u32 = 0x2;
pub const TPM_NONOTIFY: u32 = 0x80;
pub const TPM_RETURNCMD: u32 = 0x100;

pub const NIM_ADD: u32 = 0;
pub const NIM_MODIFY: u32 = 1;
pub const NIM_DELETE: u32 = 2;
pub const NIF_MESSAGE: u32 = 0x1;
pub const NIF_ICON: u32 = 0x2;
pub const NIF_TIP: u32 = 0x4;
pub const NIF_INFO: u32 = 0x10;
pub const NIIF_INFO: u32 = 0x1;
pub const NIIF_ERROR: u32 = 0x3;
pub const SM_CXSMICON: i32 = 49;

pub const ERROR_ALREADY_EXISTS: u32 = 183;
pub const WAIT_TIMEOUT: u32 = 0x102;
pub const CREATE_SUSPENDED: u32 = 0x4;
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const STARTF_USESTDHANDLES: u32 = 0x100;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const FILE_SHARE_READ: u32 = 0x1;
pub const FILE_SHARE_WRITE: u32 = 0x2;
pub const CREATE_ALWAYS: u32 = 2;
pub const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
pub const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
pub const JOB_EXTENDED_LIMIT_INFO: i32 = 9;

pub const KEY_QUERY_VALUE: u32 = 0x1;
pub const KEY_SET_VALUE: u32 = 0x2;
pub const REG_SZ: u32 = 1;

pub fn invalid() -> HANDLE {
    -1isize as HANDLE
}

pub fn hkcu() -> HKEY {
    (0x8000_0001u32 as i32) as isize as HKEY
}

#[repr(C)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
pub struct WNDCLASSW {
    pub style: u32,
    pub wndproc: Option<WNDPROC>,
    pub cls_extra: i32,
    pub wnd_extra: i32,
    pub instance: HINSTANCE,
    pub icon: HICON,
    pub cursor: HANDLE,
    pub background: HANDLE,
    pub menu_name: *const u16,
    pub class_name: *const u16,
}

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub size: u32,
    pub hwnd: HWND,
    pub id: u32,
    pub flags: u32,
    pub callback: u32,
    pub icon: HICON,
    pub tip: [u16; 128],
    pub state: u32,
    pub state_mask: u32,
    pub info: [u16; 256],
    pub timeout: u32,
    pub info_title: [u16; 64],
    pub info_flags: u32,
    pub guid: [u32; 4],
    pub balloon_icon: HICON,
}

#[repr(C)]
pub struct SECURITY_ATTRIBUTES {
    pub len: u32,
    pub sd: *mut c_void,
    pub inherit: i32,
}

#[repr(C)]
pub struct STARTUPINFOW {
    pub cb: u32,
    pub reserved: *mut u16,
    pub desktop: *mut u16,
    pub title: *mut u16,
    pub dims: [u32; 7],
    pub flags: u32,
    pub show: u16,
    pub reserved2_len: u16,
    pub reserved2: *mut u8,
    pub std_in: HANDLE,
    pub std_out: HANDLE,
    pub std_err: HANDLE,
}

#[repr(C)]
pub struct PROCESS_INFORMATION {
    pub process: HANDLE,
    pub thread: HANDLE,
    pub pid: u32,
    pub tid: u32,
}

#[repr(C)]
pub struct JOBINFO {
    pub times: [i64; 2],
    pub flags: u32,
    pub rest: [u8; 124],
}

#[repr(C)]
pub struct OPENFILENAMEW {
    pub size: u32,
    pub owner: HWND,
    pub instance: HINSTANCE,
    pub filter: *const u16,
    pub custom_filter: *mut u16,
    pub max_custom_filter: u32,
    pub filter_index: u32,
    pub file: *mut u16,
    pub max_file: u32,
    pub file_title: *mut u16,
    pub max_file_title: u32,
    pub initial_dir: *const u16,
    pub title: *const u16,
    pub flags: u32,
    pub file_offset: u16,
    pub file_extension: u16,
    pub def_ext: *const u16,
    pub cust_data: isize,
    pub hook: *const c_void,
    pub template: *const u16,
    pub reserved: *mut c_void,
    pub reserved2: u32,
    pub flags_ex: u32,
}

pub const OFN_NOCHANGEDIR: u32 = 0x8;
pub const OFN_FILEMUSTEXIST: u32 = 0x1000;
pub const OFN_EXPLORER: u32 = 0x80000;

#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(std::mem::size_of::<WNDCLASSW>() == 72);
    assert!(std::mem::size_of::<NOTIFYICONDATAW>() == 976);
    assert!(std::mem::size_of::<STARTUPINFOW>() == 104);
    assert!(std::mem::size_of::<PROCESS_INFORMATION>() == 24);
    assert!(std::mem::size_of::<JOBINFO>() == 144);
    assert!(std::mem::size_of::<SECURITY_ATTRIBUTES>() == 24);
    assert!(std::mem::size_of::<OPENFILENAMEW>() == 152);
};

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassW(wc: *const WNDCLASSW) -> u16;
    pub fn CreateWindowExW(ex: u32, cls: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: HWND, menu: HMENU, inst: HINSTANCE, param: *const c_void) -> HWND;
    pub fn DefWindowProcW(h: HWND, m: u32, wp: usize, lp: isize) -> isize;
    pub fn DestroyWindow(h: HWND) -> i32;
    pub fn PostMessageW(h: HWND, m: u32, wp: usize, lp: isize) -> i32;
    pub fn RegisterWindowMessageW(name: *const u16) -> u32;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(m: HMENU, flags: u32, id: usize, text: *const u16) -> i32;
    pub fn TrackPopupMenu(m: HMENU, flags: u32, x: i32, y: i32, r: i32, h: HWND, rc: *const c_void) -> i32;
    pub fn DestroyMenu(m: HMENU) -> i32;
    pub fn GetCursorPos(pt: *mut POINT) -> i32;
    pub fn SetForegroundWindow(h: HWND) -> i32;
    pub fn CreateIcon(inst: HINSTANCE, w: i32, h: i32, planes: u8, bits: u8, and: *const u8, xor: *const u8) -> HICON;
    pub fn DestroyIcon(i: HICON) -> i32;
    pub fn GetSystemMetrics(i: i32) -> i32;
    pub fn FindWindowW(cls: *const u16, name: *const u16) -> HWND;
}

#[link(name = "comdlg32")]
extern "system" {
    pub fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(msg: u32, data: *const NOTIFYICONDATAW) -> i32;
    pub fn ShellExecuteW(h: HWND, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> HINSTANCE;
    pub fn IsUserAnAdmin() -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    pub fn GetLastError() -> u32;
    pub fn Sleep(ms: u32);
    pub fn CloseHandle(h: HANDLE) -> i32;
    pub fn CreateMutexW(sa: *const c_void, owner: i32, name: *const u16) -> HANDLE;
    pub fn CreateFileW(name: *const u16, access: u32, share: u32, sa: *const SECURITY_ATTRIBUTES, disp: u32, flags: u32, tpl: HANDLE) -> HANDLE;
    pub fn CreateProcessW(app: *const u16, cmd: *mut u16, pa: *const c_void, ta: *const c_void, inherit: i32, flags: u32, env: *const c_void, dir: *const u16, si: *const STARTUPINFOW, pi: *mut PROCESS_INFORMATION) -> i32;
    pub fn ResumeThread(h: HANDLE) -> u32;
    pub fn TerminateProcess(h: HANDLE, code: u32) -> i32;
    pub fn WaitForSingleObject(h: HANDLE, ms: u32) -> u32;
    pub fn CreateJobObjectW(sa: *const c_void, name: *const u16) -> HANDLE;
    pub fn SetInformationJobObject(job: HANDLE, class: i32, info: *const c_void, len: u32) -> i32;
    pub fn AssignProcessToJobObject(job: HANDLE, p: HANDLE) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(k: HKEY, sub: *const u16, opts: u32, sam: u32, out: *mut HKEY) -> i32;
    pub fn RegQueryValueExW(k: HKEY, name: *const u16, res: *mut u32, ty: *mut u32, data: *mut u8, len: *mut u32) -> i32;
    pub fn RegSetValueExW(k: HKEY, name: *const u16, res: u32, ty: u32, data: *const u8, len: u32) -> i32;
    pub fn RegDeleteValueW(k: HKEY, name: *const u16) -> i32;
    pub fn RegCloseKey(k: HKEY) -> i32;
}

#[link(name = "wininet")]
extern "system" {
    pub fn InternetSetOptionW(h: *mut c_void, opt: u32, buf: *mut c_void, len: u32) -> i32;
}

#[link(name = "winhttp")]
extern "system" {
    pub fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> *mut c_void;
    pub fn WinHttpSetTimeouts(h: *mut c_void, resolve: i32, connect: i32, send: i32, recv: i32) -> i32;
    pub fn WinHttpSetOption(h: *mut c_void, opt: u32, buf: *const c_void, len: u32) -> i32;
    pub fn WinHttpConnect(h: *mut c_void, host: *const u16, port: u16, res: u32) -> *mut c_void;
    pub fn WinHttpOpenRequest(h: *mut c_void, verb: *const u16, obj: *const u16, ver: *const u16, referrer: *const u16, accept: *const *const u16, flags: u32) -> *mut c_void;
    pub fn WinHttpSendRequest(h: *mut c_void, hdr: *const u16, hdr_len: u32, opt: *const c_void, opt_len: u32, total: u32, ctx: usize) -> i32;
    pub fn WinHttpReceiveResponse(h: *mut c_void, res: *mut c_void) -> i32;
    pub fn WinHttpQueryHeaders(h: *mut c_void, info: u32, name: *const u16, buf: *mut c_void, len: *mut u32, idx: *mut u32) -> i32;
    pub fn WinHttpReadData(h: *mut c_void, buf: *mut c_void, len: u32, read: *mut u32) -> i32;
    pub fn WinHttpCloseHandle(h: *mut c_void) -> i32;
}

pub const BCRYPT_SHA256: HANDLE = 0x41 as HANDLE;

#[link(name = "bcrypt")]
extern "system" {
    pub fn BCryptHash(alg: HANDLE, key: *const u8, key_len: u32, data: *const u8, len: u32, out: *mut u8, out_len: u32) -> i32;
}
