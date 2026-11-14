use crate::api::Api;
use crate::ffi::*;
use crate::util::{base, exe, free_port, secret, w, wp, work};
use std::fs;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

pub struct Core {
    job: HANDLE,
    proc_: HANDLE,
    ctl: u16,
    secret: String,
}

impl Core {
    pub fn new() -> Self {
        let job = unsafe { CreateJobObjectW(null(), null()) };
        if !job.is_null() {
            let mut info: JOBINFO = unsafe { zeroed() };
            info.flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            unsafe { SetInformationJobObject(job, JOB_EXTENDED_LIMIT_INFO, &info as *const _ as _, size_of::<JOBINFO>() as u32) };
        }
        Self { job, proc_: null_mut(), ctl: 0, secret: String::new() }
    }

    pub fn exe() -> Option<PathBuf> {
        let b = base();
        for p in [b.join("mihomo.exe"), work().join("mihomo.exe")] {
            if p.is_file() {
                return Some(p);
            }
        }
        let me = exe();
        fs::read_dir(&b).ok()?.flatten().map(|e| e.path()).find(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
            *p != me && n.starts_with("mihomo") && n.ends_with(".exe")
        })
    }

    pub fn start(&mut self, profile: &Path) -> Result<(), String> {
        self.stop();
        let bin = Self::exe().ok_or("mihomo.exe не найден рядом с программой")?;
        if !profile.is_file() {
            return Err(format!("Нет профиля {}", profile.display()));
        }
        let wd = work();
        fs::create_dir_all(&wd).map_err(|e| e.to_string())?;
        self.ctl = free_port();
        self.secret = secret();
        let mut cmd = w(&format!(
            "\"{}\" -d \"{}\" -f \"{}\" -ext-ctl 127.0.0.1:{} -secret {}",
            bin.display(),
            wd.display(),
            profile.display(),
            self.ctl,
            self.secret
        ));
        let dir = wp(&wd);
        let sa = SECURITY_ATTRIBUTES { len: size_of::<SECURITY_ATTRIBUTES>() as u32, sd: null_mut(), inherit: 1 };
        unsafe {
            let log = CreateFileW(wp(&wd.join("mihomo.log")).as_ptr(), GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, &sa, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, null_mut());
            let has = log != invalid();
            let mut si: STARTUPINFOW = zeroed();
            si.cb = size_of::<STARTUPINFOW>() as u32;
            if has {
                si.flags = STARTF_USESTDHANDLES;
                si.std_out = log;
                si.std_err = log;
            }
            let mut pi: PROCESS_INFORMATION = zeroed();
            let ok = CreateProcessW(null(), cmd.as_mut_ptr(), null(), null(), has as i32, CREATE_NO_WINDOW | CREATE_SUSPENDED, null(), dir.as_ptr(), &si, &mut pi);
            let err = std::io::Error::last_os_error();
            if has {
                CloseHandle(log);
            }
            if ok == 0 {
                return Err(format!("Запуск ядра: {err}"));
            }
            if !self.job.is_null() {
                AssignProcessToJobObject(self.job, pi.process);
            }
            ResumeThread(pi.thread);
            CloseHandle(pi.thread);
            self.proc_ = pi.process;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.proc_.is_null() {
            return;
        }
        unsafe {
            TerminateProcess(self.proc_, 0);
            WaitForSingleObject(self.proc_, 3000);
            CloseHandle(self.proc_);
        }
        self.proc_ = null_mut();
    }

    pub fn alive(&self) -> bool {
        !self.proc_.is_null() && unsafe { WaitForSingleObject(self.proc_, 0) } == WAIT_TIMEOUT
    }

    pub fn api(&self) -> Api {
        Api { port: self.ctl, secret: self.secret.clone() }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        self.stop();
        if !self.job.is_null() {
            unsafe { CloseHandle(self.job) };
        }
    }
}
