//! One icon per session: the copy Windows starts at logon and one opened by
//! hand would otherwise both sit in the notification area.

/// Holds the session-wide mutex for as long as it lives.
pub struct Guard(#[cfg(windows)] windows_sys::Win32::Foundation::HANDLE);

/// The guard when no other icon runs in this session, `None` when one does.
pub fn acquire() -> Option<Guard> {
    acquire_named(r"Local\MeshLLMTray")
}

#[cfg(windows)]
fn acquire_named(name: &str) -> Option<Guard> {
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `name` is a NUL-terminated UTF-16 string that outlives the call.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    // SAFETY: reads the calling thread's last error, set by CreateMutexW.
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if handle.is_null() || already {
        if !handle.is_null() {
            drop(Guard(handle));
        }
        return None;
    }
    Some(Guard(handle))
}

#[cfg(not(windows))]
fn acquire_named(_name: &str) -> Option<Guard> {
    Some(Guard())
}

#[cfg(windows)]
impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateMutexW and is closed once.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn a_second_guard_is_refused_while_the_first_lives() {
        // Not the real name, so a running icon does not fail the test.
        let name = format!(r"Local\MeshLLMTrayTest{}", std::process::id());
        let first = super::acquire_named(&name).expect("first guard");
        assert!(super::acquire_named(&name).is_none());
        drop(first);
        assert!(super::acquire_named(&name).is_some());
    }
}
