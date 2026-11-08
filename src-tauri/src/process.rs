use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;
use std::process::Stdio;

use tokio::process::{Child, Command};

pub use platform::ProcessTree;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn spawn(program: &Path, args: &[OsString]) -> io::Result<(Child, ProcessTree)> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let child = command.spawn()?;
    let tree = ProcessTree::attach(&child)?;
    Ok((child, tree))
}

pub fn quiet_command(program: impl AsRef<OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut command = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(unix)]
mod platform {
    use std::io;

    use tokio::process::Child;

    #[derive(Clone)]
    pub struct ProcessTree {
        group: libc::pid_t,
    }

    impl ProcessTree {
        pub fn attach(child: &Child) -> io::Result<Self> {
            let id = child
                .id()
                .ok_or_else(|| io::Error::other("process exited before it could be tracked"))?;
            Ok(Self {
                group: id as libc::pid_t,
            })
        }

        pub fn kill(&self) {
            unsafe {
                libc::killpg(self.group, libc::SIGKILL);
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::sync::Arc;
    use std::{io, mem, ptr};

    use tokio::process::Child;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    struct JobHandle(HANDLE);

    unsafe impl Send for JobHandle {}
    unsafe impl Sync for JobHandle {}

    impl Drop for JobHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    #[derive(Clone)]
    pub struct ProcessTree {
        job: Arc<JobHandle>,
    }

    impl ProcessTree {
        pub fn attach(child: &Child) -> io::Result<Self> {
            let process = child
                .raw_handle()
                .ok_or_else(|| io::Error::other("process exited before it could be tracked"))?;
            unsafe {
                let handle = CreateJobObjectW(ptr::null(), ptr::null());
                if handle.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let job = JobHandle(handle);
                let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let configured = SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    (&raw const limits).cast(),
                    mem::size_of_val(&limits) as u32,
                );
                if configured == 0 || AssignProcessToJobObject(job.0, process as HANDLE) == 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(Self { job: Arc::new(job) })
            }
        }

        pub fn kill(&self) {
            unsafe {
                TerminateJobObject(self.job.0, 1);
            }
        }
    }
}
