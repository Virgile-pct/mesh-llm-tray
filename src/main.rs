//! MeshLLM in the Windows notification area: the logo says whether the local
//! node is up, and a menu opens its console, starts or stops it.
//! Everything goes through what a person would use: the console port, the
//! `mesh-llm` scheduled task and `mesh-llm stop`.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod autostart;
mod icon;
mod node;

use node::NodeState;
use std::time::{Duration, Instant};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, DeviceEventFilter, EventLoopBuilder};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

const POLL: Duration = Duration::from_secs(3);

enum UserEvent {
    Menu(MenuEvent),
}

struct Items {
    status: MenuItem,
    open: MenuItem,
    start: MenuItem,
    stop: MenuItem,
    autostart: CheckMenuItem,
    quit: MenuItem,
}

fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // Raw mouse and keyboard input from the whole desktop is of no use here,
    // and every such event would wake the loop.
    event_loop.set_device_event_filter(DeviceEventFilter::Always);
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let items = Items {
        status: MenuItem::new("MeshLLM", false, None),
        open: MenuItem::new("Open console", false, None),
        start: MenuItem::new("Start node", false, None),
        stop: MenuItem::new("Stop node", false, None),
        autostart: CheckMenuItem::new("Start with Windows", true, autostart::is_enabled(), None),
        quit: MenuItem::new("Quit", true, None),
    };
    let mut tray: Option<TrayIcon> = None;
    let mut shown: Option<NodeState> = None;
    // A fixed deadline: events that wake the loop in between must not push
    // the next probe back, or a busy desktop would never refresh the icon.
    let mut next_poll = Instant::now() + POLL;

    event_loop.run(move |event, _, control_flow| {
        if Instant::now() >= next_poll {
            refresh(tray.as_ref(), &items, &mut shown);
            next_poll = Instant::now() + POLL;
        }
        *control_flow = ControlFlow::WaitUntil(next_poll);
        match event {
            Event::NewEvents(StartCause::Init) => {
                let menu = Menu::new();
                let _ = menu.append_items(&[
                    &items.status,
                    &PredefinedMenuItem::separator(),
                    &items.open,
                    &items.start,
                    &items.stop,
                    &PredefinedMenuItem::separator(),
                    &items.autostart,
                    &items.quit,
                ]);
                tray = TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_tooltip("MeshLLM")
                    .with_icon(icon::for_state(NodeState::Stopped))
                    .build()
                    .inspect_err(|error| log(&format!("tray icon not created: {error}")))
                    .ok();
                log(&format!("tray icon created: {}", tray.is_some()));
                refresh(tray.as_ref(), &items, &mut shown);
            }
            Event::UserEvent(UserEvent::Menu(event)) => {
                if event.id == items.open.id() {
                    node::open_console();
                } else if event.id == items.start.id() {
                    node::start();
                } else if event.id == items.stop.id() {
                    node::stop();
                } else if event.id == items.autostart.id() {
                    // The menu already flipped the check; keep it only if the
                    // registry agrees.
                    let wanted = items.autostart.is_checked();
                    if !autostart::set(wanted) {
                        log(&format!("start with Windows: could not set {wanted}"));
                    }
                    items.autostart.set_checked(autostart::is_enabled());
                } else if event.id == items.quit.id() {
                    tray.take();
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}

/// Probe the node and, when its state changed, update the icon, tooltip and menu.
fn refresh(tray: Option<&TrayIcon>, items: &Items, shown: &mut Option<NodeState>) {
    let state = node::probe();
    let service = node::service_installed();
    let running = state == NodeState::Running;
    items.open.set_enabled(running);
    items
        .stop
        .set_enabled(running && node::mesh_llm_exe().is_some());
    items.start.set_enabled(!running && service);
    let label = match (state, service) {
        (NodeState::Running, _) => "MeshLLM is running",
        (NodeState::Stopped, true) => "MeshLLM is stopped",
        (NodeState::Stopped, false) => "MeshLLM is stopped (run `mesh-llm setup` first)",
    };
    items.status.set_text(label);
    if *shown != Some(state) {
        if let Some(tray) = tray {
            let _ = tray.set_icon(Some(icon::for_state(state)));
            let _ = tray.set_tooltip(Some(label));
        }
        *shown = Some(state);
    }
}

/// Append a line to %TEMP%\mesh-llm-tray.log; the app has no console to print to.
fn log(line: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("mesh-llm-tray.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}
