//! A private, non-breakaway job owns each shell and its descendants.
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use tokio::process::{Child, Command};
use windows_sys::Win32::Foundation::{HANDLE, NTSTATUS};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtResumeProcess(process: HANDLE) -> NTSTATUS;
}

pub(super) struct JobObject(OwnedHandle);

impl JobObject {
    pub(super) fn new() -> io::Result<Self> {
        // SAFETY: null attributes create a non-inheritable, unnamed job.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: CreateJobObjectW returned a fresh handle owned by this value.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(handle) });
        // SAFETY: this Windows structure permits zero initialization.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the handle is valid and the pointer/size describe the limits structure.
        let configured = unsafe {
            SetInformationJobObject(
                job.0.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(limits).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        };
        if configured == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    pub(super) fn spawn(&self, command: &mut Command) -> io::Result<Child> {
        // Suspension prevents descendants from escaping before assignment. Never
        // resume or retry without containment if the host job forbids nesting.
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED).kill_on_drop(true);
        let mut child = command.spawn()?;
        let result = (|| {
            let process =
                child.raw_handle().ok_or_else(|| io::Error::other("Missing shell handle"))?;
            // SAFETY: both handles remain alive throughout assignment and resume.
            if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), process) } == 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: this process was created suspended and belongs to our job.
            let status = unsafe { NtResumeProcess(process) };
            if status < 0 {
                return Err(io::Error::other(format!(
                    "Failed to resume contained shell process: NTSTATUS {status:#x}"
                )));
            }
            Ok(())
        })();
        if let Err(error) = result {
            // No user code has run. Kill the suspended root even if assignment failed.
            let _ = child.start_kill();
            return Err(error);
        }
        Ok(child)
    }

    pub(super) fn terminate(&self) -> io::Result<()> {
        // SAFETY: the owned handle identifies only this command's private job.
        if unsafe { TerminateJobObject(self.0.as_raw_handle(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}
