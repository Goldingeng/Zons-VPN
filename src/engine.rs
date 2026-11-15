use crate::api::Api;
use crate::ffi::*;
use crate::util::{base, exe, free_port, secret, work};
use std::fs;
use std::mem::{size_of, zeroed};
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::ptr::null;

pub struct Core {
    job: HANDLE,
    child: Option<Child>,
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
        Self { job, child: None, ctl: 0, secret: String::new() }
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
        let log = wd.join("mihomo.log");
        let _ = fs::rename(&log, wd.join("mihomo.old.log"));
        let (out, err) = match fs::File::create(&log).and_then(|f| Ok((f.try_clone()?, f))) {
            Ok((a, b)) => (Stdio::from(a), Stdio::from(b)),
            Err(_) => (Stdio::null(), Stdio::null()),
        };
        let child = Command::new(&bin)
            .arg("-d")
            .arg(&wd)
            .arg("-f")
            .arg(profile)
            .arg("-ext-ctl")
            .arg(format!("127.0.0.1:{}", self.ctl))
            .arg("-secret")
            .arg(&self.secret)
            .current_dir(&wd)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Запуск ядра: {e}"))?;
        if !self.job.is_null() {
            unsafe { AssignProcessToJobObject(self.job, child.as_raw_handle() as HANDLE) };
        }
        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        let Some(mut c) = self.child.take() else { return };
        let _ = c.kill();
        unsafe { WaitForSingleObject(c.as_raw_handle() as HANDLE, 3000) };
    }

    pub fn alive(&self) -> bool {
        self.child.as_ref().is_some_and(|c| unsafe { WaitForSingleObject(c.as_raw_handle() as HANDLE, 0) } == WAIT_TIMEOUT)
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
