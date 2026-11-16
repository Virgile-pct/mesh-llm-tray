//! The local MeshLLM node, seen from outside: whether it answers, and the
//! commands a person would type to open, start or stop it.

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// The web console `mesh-llm` serves by default.
pub const CONSOLE_URL: &str = "http://localhost:3131";
const CONSOLE_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 3131);
/// The scheduled task `mesh-llm setup --service` installs on Windows.
const SERVICE_TASK: &str = "mesh-llm";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState {
    /// The console answers on its port.
    Running,
    /// Nothing answers.
    Stopped,
}

/// Whether the node's console accepts a connection right now.
pub fn probe() -> NodeState {
    let addr = SocketAddr::from(CONSOLE_ADDR);
    match TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
        Ok(_) => NodeState::Running,
        Err(_) => NodeState::Stopped,
    }
}

/// Whether setup installed the background service, so Start has something to run.
pub fn service_installed() -> bool {
    hidden("schtasks")
        .args(["/Query", "/TN", SERVICE_TASK])
        .status()
        .is_ok_and(|status| status.success())
}

/// Open the console in the default browser.
pub fn open_console() {
    let _ = hidden("rundll32")
        .args(["url.dll,FileProtocolHandler", CONSOLE_URL])
        .spawn();
}

/// Start the background service setup installed.
pub fn start() {
    let _ = hidden("schtasks").args(["/Run", "/TN", SERVICE_TASK]).spawn();
}

/// Stop the node the way a person would: `mesh-llm stop` ends the service
/// first, so its supervisor cannot restart the node.
pub fn stop() {
    if let Some(exe) = mesh_llm_exe() {
        let _ = hidden(exe).arg("stop").spawn();
    }
}

/// The `mesh-llm.exe` to drive the node with: the very binary the node runs
/// (its `owner.json` records it), so an older install on PATH never gets to
/// stop a newer node; else the one on PATH, else install.ps1's default.
pub fn mesh_llm_exe() -> Option<PathBuf> {
    running_node_exe().or_else(installed_exe)
}

/// The binary of the newest runtime slot that serves this console port.
fn running_node_exe() -> Option<PathBuf> {
    let root = std::env::var_os("MESH_LLM_RUNTIME_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|home| PathBuf::from(home).join(".mesh-llm").join("runtime"))
        })?;
    std::fs::read_dir(root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let owner = entry.path().join("owner.json");
            let modified = std::fs::metadata(&owner).and_then(|m| m.modified()).ok()?;
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&owner).ok()?).ok()?;
            (json.get("api_port")?.as_u64()? == u64::from(CONSOLE_ADDR.1)).then_some(())?;
            let exe = PathBuf::from(json.get("mesh_llm_binary")?.as_str()?);
            exe.is_file().then_some((modified, exe))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, exe)| exe)
}

/// `mesh-llm.exe` on PATH, else where install.ps1 puts it by default.
fn installed_exe() -> Option<PathBuf> {
    let on_path = std::env::var_os("PATH").into_iter().flat_map(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join("mesh-llm.exe"))
            .collect::<Vec<_>>()
    });
    let default = std::env::var_os("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("mesh-llm").join("bin").join("mesh-llm.exe"));
    on_path.chain(default).find(|candidate| candidate.is_file())
}

/// A command that never flashes a console window.
fn hidden(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn a_listening_console_reads_as_running() {
        // Only meaningful when 3131 is free; skip on a machine running a node.
        let Ok(listener) = TcpListener::bind(SocketAddr::from(CONSOLE_ADDR)) else {
            return;
        };
        assert_eq!(probe(), NodeState::Running);
        drop(listener);
        assert_eq!(probe(), NodeState::Stopped);
    }
}
