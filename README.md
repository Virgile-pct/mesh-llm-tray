# MeshLLM tray

A Windows notification-area icon for a local [MeshLLM](https://github.com/Mesh-LLM/mesh-llm) node, for people who installed it with `install.ps1` and would rather not keep a terminal open.

- The MeshLLM logo in colour with a green badge while the node answers on `localhost:3131`, greyed otherwise. It checks every 3 seconds.
- **Open console** opens the web console in the default browser.
- **Start node** runs the background service `mesh-llm setup --service` installed (the `mesh-llm` scheduled task).
- **Stop node** runs `mesh-llm stop` with the node's own `mesh-llm.exe` (read from its `owner.json`), so an older install on `PATH` never handles a newer node. `stop` ends the service first, so it does not restart the node.
- **Start with Windows** copies the icon to `%LOCALAPPDATA%\Programs\MeshLLM Tray\` and adds a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` that starts that copy, the list Windows' startup apps settings show. The item is ticked only when that value starts the copy, so a value left pointing at a moved file shows unticked and ticking it repairs it.
- One icon per session: starting it again while one runs does nothing.

It only uses what a person would: the console port, the scheduled task and the `mesh-llm` command. No console window opens.

## Install

Download `mesh-llm-tray.exe` from the latest release, check it against the `.sha256` next to it, run it, and tick **Start with Windows** in its menu. The downloaded file can go after that. Windows 11 puts new icons behind the `^` arrow of the taskbar; drag it out to keep it visible.

To update, quit the running icon from its menu, run the new version and tick **Start with Windows** again.

## Build

```bash
cargo build --release
```

Tested on Windows 11 with MeshLLM 0.80.0: status on a CPU node, Stop and Start against the real background service.

## Licence

MIT or Apache-2.0, at your option. The logo is the MeshLLM console favicon from [Mesh-LLM/mesh-llm](https://github.com/Mesh-LLM/mesh-llm) (Apache-2.0).
