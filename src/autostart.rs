//! "Start with Windows": a value under the user's Run key, the same place
//! Windows' own startup apps settings list and can switch off.
//!
//! The value points at a copy of the executable under
//! `%LOCALAPPDATA%\Programs\MeshLLM Tray`, where per-user apps live, so the
//! icon still starts after the downloaded file is moved or deleted.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "MeshLLM tray";
const INSTALL_DIR: &str = r"Programs\MeshLLM Tray";
const EXE_NAME: &str = "mesh-llm-tray.exe";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Ticked only when the value starts the stable copy: a value left by an
/// older version, or pointing at a build folder or a download, is not the
/// setup this menu item promises, and ticking it again repairs it.
pub fn is_enabled() -> bool {
    match (registered_command(), stable_exe()) {
        (Some(command), Some(stable)) => starts(&command, &stable),
        _ => false,
    }
}

/// Whether a Run command line starts `exe`, quoted or not.
fn starts(command: &str, exe: &Path) -> bool {
    same_path(Path::new(command.trim().trim_matches('"')), exe)
}

/// The Run value's command line, read through the registry API rather than
/// `reg query`, whose output follows the console code page.
#[cfg(windows)]
fn registered_command() -> Option<String> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};

    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() };
    let subkey = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let value = wide(VALUE);
    let mut buffer = vec![0u16; 1024];
    let mut bytes = (buffer.len() * 2) as u32;
    // SAFETY: both names are NUL-terminated UTF-16, `buffer` holds `bytes`
    // bytes, and the call writes at most that many.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if status != 0 {
        return None;
    }
    // `bytes` counts the terminating NUL.
    let len = (bytes as usize / 2).saturating_sub(1).min(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

#[cfg(not(windows))]
fn registered_command() -> Option<String> {
    None
}

/// Register or remove the stable copy; true when the change took.
pub fn set(enabled: bool) -> bool {
    if !enabled {
        return reg(&["delete", RUN_KEY, "/v", VALUE, "/f"]);
    }
    let Some(target) = stable_exe() else {
        return false;
    };
    if !install_copy(&target) {
        return false;
    }
    let quoted = format!("\"{}\"", target.display());
    reg(&[
        "add", RUN_KEY, "/v", VALUE, "/t", "REG_SZ", "/d", &quoted, "/f",
    ])
}

/// Where Windows starts the icon from.
pub fn stable_exe() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(Path::new(&local).join(INSTALL_DIR).join(EXE_NAME))
}

/// Put this executable at `target`, unless it is already running from there.
/// A copy that is running (and so locked) is kept as it is.
fn install_copy(target: &Path) -> bool {
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    if same_file(&current, target) {
        return true;
    }
    let Some(dir) = target.parent() else {
        return false;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    std::fs::copy(&current, target).is_ok() || target.is_file()
}

/// Windows paths compare without regard to case, and `current_exe` may come
/// back in a different form than the one built here.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => same_path(&a, &b),
        _ => same_path(a, b),
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
}

fn reg(args: &[&str]) -> bool {
    let mut command = Command::new("reg");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.status().is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stable_copy_lives_under_local_programs() {
        let exe = stable_exe().expect("LOCALAPPDATA is set on Windows");
        assert!(
            exe.ends_with(r"Programs\MeshLLM Tray\mesh-llm-tray.exe"),
            "{exe:?}"
        );
    }

    #[test]
    fn only_a_command_that_starts_the_stable_copy_counts() {
        let stable =
            Path::new(r"C:\Users\Jane\AppData\Local\Programs\MeshLLM Tray\mesh-llm-tray.exe");
        assert!(starts(
            r#""C:\Users\Jane\AppData\Local\Programs\MeshLLM Tray\mesh-llm-tray.exe""#,
            stable
        ));
        assert!(starts(
            r"C:\Users\Jane\AppData\Local\Programs\MeshLLM Tray\mesh-llm-tray.exe",
            stable
        ));
        assert!(!starts(
            r#""C:\Users\Jane\dev\mesh-llm-tray\target\debug\mesh-llm-tray.exe""#,
            stable
        ));
    }

    #[test]
    fn paths_compare_without_case() {
        assert!(same_path(
            Path::new(r"C:\Users\Jane\AppData\Local\Programs\MeshLLM Tray\mesh-llm-tray.exe"),
            Path::new(r"c:\users\jane\appdata\local\programs\meshllm tray\MESH-LLM-TRAY.EXE"),
        ));
        assert!(!same_path(
            Path::new(r"C:\Users\Jane\Downloads\mesh-llm-tray.exe"),
            Path::new(r"C:\Users\Jane\AppData\Local\Programs\MeshLLM Tray\mesh-llm-tray.exe"),
        ));
    }

    #[test]
    fn installing_copies_the_running_exe_and_keeps_a_copy_already_there() {
        let dir = std::env::temp_dir().join(format!("mesh-llm-tray-test-{}", std::process::id()));
        let target = dir.join("nested").join(EXE_NAME);
        assert!(install_copy(&target));
        let current = std::env::current_exe().unwrap();
        assert_eq!(
            std::fs::metadata(&target).unwrap().len(),
            std::fs::metadata(&current).unwrap().len()
        );
        // Running from the copy itself installs nothing more.
        assert!(same_file(&target, &target));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
