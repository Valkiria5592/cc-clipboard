# cc-clipboard

A Windows system tray app that captures screen regions, saves them to a configured folder, and auto-copies the file path to clipboard — built for use with Claude Code.

## Commit rules

- **Never** include `Co-Authored-By: Claude` or any AI attribution line in commit messages.
- **Never** mention "Claude Code", "Anthropic", or any AI tool in commit messages.
- Commit messages must be plain, human-readable, and follow the format: `type: short description`.
- Valid types: `feat`, `fix`, `chore`, `refactor`, `build`, `docs`, `test`.

## Project structure

```
cc-clipboard/
├── src/
│   ├── main.rs          # Entry point + winit event loop
│   ├── tray.rs          # System tray icon and context menu
│   ├── hotkey.rs        # Global hotkey registration
│   ├── capture.rs       # Full-screen xcap capture + crop logic
│   ├── overlay.rs       # Fullscreen region-selection overlay window
│   ├── clipboard.rs     # Arboard clipboard write
│   ├── notification.rs  # Win toast notifications
│   └── config.rs        # Serde config load/save (AppData)
├── assets/
│   └── icon.ico
├── build.rs             # Windows subsystem (hides console)
└── Cargo.toml
```

## Dev commands

```powershell
cargo build               # debug build
cargo build --release     # release build
cargo run                 # run in debug mode
```

## Default hotkey

`Ctrl + Shift + S` — triggers region selection overlay from anywhere.

## Default save folder

`%USERPROFILE%\Pictures\cc-clipboard\`
