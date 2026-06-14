#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod clipboard;
mod config;
mod hotkey;
mod notification;
mod overlay;

use config::Config;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use overlay::OverlayState;
use std::sync::Arc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Cursor, CursorIcon, Fullscreen, WindowAttributes, WindowId},
};

// ── tray icon: crab from logo.jpeg, center-cropped to 32×32 ──────────────────

fn make_icon() -> tray_icon::Icon {
    use image::{imageops::FilterType, GenericImageView};
    let bytes = include_bytes!("../assets/logo.jpeg");
    let img = image::load_from_memory(bytes).expect("logo.jpeg");
    let (w, h) = img.dimensions();
    let size = w.min(h);
    let x = (w - size) / 2;
    let y = (h - size) / 2;
    let rgba = img
        .crop_imm(x, y, size, size)
        .resize(32, 32, FilterType::Lanczos3)
        .to_rgba8();
    tray_icon::Icon::from_rgba(rgba.into_raw(), 32, 32).expect("icon")
}

// ── App state ─────────────────────────────────────────────────────────────────

struct MenuIds {
    screenshot: tray_icon::menu::MenuId,
    open_folder: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

struct App {
    config: Config,
    // Kept alive so the tray icon stays visible
    _tray: Option<TrayIcon>,
    menu_ids: Option<MenuIds>,
    // Kept alive to maintain hotkey registration
    _hotkey_manager: Option<GlobalHotKeyManager>,
    hotkey_id: u32,
    overlays: Vec<OverlayState>,
}

impl App {
    fn new() -> Self {
        App {
            config: Config::load(),
            _tray: None,
            menu_ids: None,
            _hotkey_manager: None,
            hotkey_id: 0,
            overlays: Vec::new(),
        }
    }

    fn start_capture(&mut self, event_loop: &ActiveEventLoop) {
        if !self.overlays.is_empty() {
            return;
        }

        let captures = capture::capture_all();
        if captures.is_empty() {
            return;
        }

        // Build winit monitor list for matching by position
        let winit_monitors: Vec<_> = event_loop.available_monitors().collect();

        for (img, xcap_x, xcap_y, mon_w, mon_h) in captures {
            // Match xcap monitor to winit monitor by top-left position
            let winit_mon = winit_monitors
                .iter()
                .find(|m| {
                    let pos = m.position();
                    pos.x == xcap_x && pos.y == xcap_y
                })
                .cloned();

            let attrs = WindowAttributes::default()
                .with_title("cc-clipboard-overlay")
                .with_decorations(false)
                .with_resizable(false)
                .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
                .with_fullscreen(Some(Fullscreen::Borderless(winit_mon)));

            let Ok(window) = event_loop.create_window(attrs) else {
                continue;
            };
            window.set_cursor(Cursor::Icon(CursorIcon::Crosshair));
            let window = Arc::new(window);

            let inner = window.inner_size();
            let (w, h) = if inner.width > 0 && inner.height > 0 {
                (inner.width, inner.height)
            } else {
                (mon_w, mon_h)
            };

            self.overlays.push(OverlayState::new(window, img, w, h));
        }
    }

    fn finish_capture(&self, img: image::RgbaImage, x1: u32, y1: u32, x2: u32, y2: u32) {
        let lx = x1.min(x2);
        let rx = x1.max(x2);
        let ty = y1.min(y2);
        let by = y1.max(y2);

        if rx.saturating_sub(lx) < 5 || by.saturating_sub(ty) < 5 {
            return;
        }

        // Run crop + clipboard + toast on a background thread so the overlay
        // window can close and repaint immediately without blocking the message pump.
        let config = self.config.clone();
        std::thread::spawn(move || {
            if let Some(path) = capture::crop_and_save(&img, lx, ty, rx - lx, by - ty, &config) {
                clipboard::copy_path(&path);
                if config.notify {
                    notification::notify_saved(&path);
                }
            }
        });
    }

    fn open_folder(&self) {
        let _ = std::process::Command::new("explorer")
            .arg(&self.config.save_folder)
            .spawn();
    }
}

// ── ApplicationHandler ────────────────────────────────────────────────────────

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Build tray context menu
        let screenshot_item = MenuItem::new("Screenshot  (Ctrl+Shift+S)", true, None);
        let open_folder_item = MenuItem::new("Open Folder", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        self.menu_ids = Some(MenuIds {
            screenshot: screenshot_item.id().clone(),
            open_folder: open_folder_item.id().clone(),
            quit: quit_item.id().clone(),
        });

        let menu = Menu::new();
        let _ = menu.append_items(&[
            &screenshot_item,
            &open_folder_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ]);

        self._tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("cc-clipboard")
            .with_icon(make_icon())
            .build()
            .ok();

        // Register global hotkey
        if let Ok(manager) = GlobalHotKeyManager::new() {
            let hk = hotkey::parse_hotkey(&self.config.hotkey);
            self.hotkey_id = hk.id();
            let _ = manager.register(hk);
            self._hotkey_manager = Some(manager);
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                if let Some(ov) = self.overlays.iter_mut().find(|o| o.window.id() == id) {
                    ov.draw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(ov) = self.overlays.iter_mut().find(|o| o.window.id() == id) {
                    ov.drag_current = Some((position.x as u32, position.y as u32));
                    ov.window.request_redraw();
                }
            }

            WindowEvent::MouseInput { button, state, .. } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        if let Some(ov) = self.overlays.iter_mut().find(|o| o.window.id() == id) {
                            ov.drag_start = ov.drag_current;
                        }
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        if let Some(idx) = self.overlays.iter().position(|o| o.window.id() == id) {
                            let ov = self.overlays.remove(idx);
                            self.overlays.clear(); // close all other monitor overlays
                            if let (Some(s), Some(c)) = (ov.drag_start, ov.drag_current) {
                                self.finish_capture(ov.screenshot, s.0, s.1, c.0, c.1);
                            }
                        }
                    }
                    // Right-click cancels all overlays
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.overlays.clear();
                    }
                    _ => {}
                }
            }

            WindowEvent::KeyboardInput { event: key, .. } => {
                use winit::keyboard::{KeyCode, PhysicalKey};
                if key.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    self.overlays.clear();
                }
            }

            // Focused(false) is intentionally not handled: clicking on the overlay of
            // a second monitor fires Focused(false) on the first, which would wrongly
            // close everything before the user even starts selecting.

            WindowEvent::CloseRequested => {
                self.overlays.clear();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Poll tray menu events
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some(ids) = &self.menu_ids {
                if ev.id == ids.screenshot {
                    self.start_capture(event_loop);
                } else if ev.id == ids.open_folder {
                    self.open_folder();
                } else if ev.id == ids.quit {
                    event_loop.exit();
                }
            }
        }

        // Poll global hotkey events
        while let Ok(ev) = GlobalHotKeyEvent::receiver().try_recv() {
            if ev.id() == self.hotkey_id && ev.state() == HotKeyState::Pressed {
                self.start_capture(event_loop);
            }
        }

        // Use Poll when overlays are open (responsive redraws), Wait otherwise (low CPU)
        if !self.overlays.is_empty() {
            event_loop.set_control_flow(ControlFlow::Poll);
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run");
}
