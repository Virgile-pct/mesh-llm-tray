//! "Start with Windows": a value under the user's Run key, the same place
//! Windows' own startup apps settings list and can switch off.

use std::process::{Command, Stdio};

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "MeshLLM tray";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn is_enabled() -> bool {
    reg(&["query", RUN_KEY, "/v", VALUE])
}

/// Register or remove this executable; true when the change took.
pub fn set(enabled: bool) -> bool {
    if !enabled {
        return reg(&["delete", RUN_KEY, "/v", VALUE, "/f"]);
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let quoted = format!("\"{}\"", exe.display());
    reg(&[
        "add", RUN_KEY, "/v", VALUE, "/t", "REG_SZ", "/d", &quoted, "/f",
    ])
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
