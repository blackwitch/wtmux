use anyhow::Result;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::ptr;
use tracing::debug;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

/// A Windows Job Object that ensures child processes are killed when dropped.
pub struct JobObject {
    handle: OwnedHandle,
}

unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}

impl JobObject {
    /// Create a new Job Object configured to kill all processes on close.
    pub fn new() -> Result<Self> {
        unsafe {
            let handle = CreateJobObjectW(ptr::null(), ptr::null());
            if handle.is_null() {
                anyhow::bail!(
                    "CreateJobObjectW failed: {}",
                    std::io::Error::last_os_error()
                );
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let success = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if success == 0 {
                CloseHandle(handle);
                anyhow::bail!(
                    "SetInformationJobObject failed: {}",
                    std::io::Error::last_os_error()
                );
            }

            debug!("Job object created");
            Ok(JobObject {
                handle: OwnedHandle::from_raw_handle(handle as _),
            })
        }
    }

    /// Assign a process to this job object.
    pub fn assign_process(&self, process_handle: HANDLE) -> Result<()> {
        use std::os::windows::io::AsRawHandle;
        unsafe {
            let success =
                AssignProcessToJobObject(self.handle.as_raw_handle() as _, process_handle);
            if success == 0 {
                anyhow::bail!(
                    "AssignProcessToJobObject failed: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
        Ok(())
    }
}
