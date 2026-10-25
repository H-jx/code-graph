//! 子进程树管理（计划 §3.5、U03）。
//!
//! - Unix：把子进程放进独立进程组，超时或取消时终止整个进程组。
//! - Windows：把子进程分配到 Job Object，并设置 `KILL_ON_JOB_CLOSE`；
//!   宿主退出或句柄关闭时后代进程一并结束。
//!
//! 终止只针对本次运行创建并拥有的进程（按 PID / Job 句柄），不按进程名匹配，
//! 避免误杀其他用户的同名进程。

use std::process::Stdio;

use tokio::process::{Child, Command};

/// 配置进程组 / Job Object。
pub fn configure_process_tree(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // 新建进程组，便于整体终止后代进程。
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    let _ = Stdio::null();
}

/// 进程树守卫：负责终止本次运行拥有的进程树。
pub struct ProcessTreeGuard {
    #[cfg(windows)]
    job: Option<windows_job::Job>,
    #[cfg(unix)]
    process_group: Option<i32>,
    pid: Option<u32>,
}

impl ProcessTreeGuard {
    /// 附着到已启动的子进程。
    pub fn attach(child: &mut Child) -> ProcessTreeGuard {
        let pid = child.id();
        #[cfg(windows)]
        {
            let job = pid.and_then(windows_job::Job::assign);
            ProcessTreeGuard {
                job,
                #[cfg(unix)]
                process_group: None,
                pid,
            }
        }
        #[cfg(unix)]
        {
            ProcessTreeGuard {
                process_group: pid.map(|value| value as i32),
                pid,
                #[cfg(windows)]
                job: None,
            }
        }
    }

    /// 终止本次运行的进程树。
    pub fn terminate(&mut self) {
        #[cfg(windows)]
        {
            if let Some(job) = self.job.as_mut() {
                job.terminate();
            }
        }
        #[cfg(unix)]
        {
            if let Some(group) = self.process_group {
                unsafe {
                    libc::killpg(group, libc::SIGKILL);
                }
            }
        }
        let _ = self.pid;
    }
}

impl Drop for ProcessTreeGuard {
    fn drop(&mut self) {
        // 句柄关闭时 Job Object 会终止剩余后代进程。
        #[cfg(windows)]
        {
            self.job.take();
        }
    }
}

#[cfg(unix)]
extern crate libc;

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows_job {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    /// Job Object 句柄。
    pub struct Job {
        handle: HANDLE,
    }

    // Job 句柄只在宿主线程使用，跨 await 时需要同步；这里用裸句柄并在 Drop 中关闭。
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        /// 把进程分配到新建的 Job Object。
        pub fn assign(pid: u32) -> Option<Job> {
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if handle.is_null() {
                    return None;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                if SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                ) == 0
                {
                    let _ = CloseHandle(handle);
                    return None;
                }
                let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
                if process.is_null() {
                    let _ = CloseHandle(handle);
                    return None;
                }
                let assigned = AssignProcessToJobObject(handle, process);
                let _ = CloseHandle(process);
                if assigned == 0 {
                    let _ = CloseHandle(handle);
                    return None;
                }
                Some(Job { handle })
            }
        }

        /// 终止 Job 中的全部进程。
        pub fn terminate(&mut self) {
            unsafe {
                let _ = TerminateJobObject(self.handle, 1);
            }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}
