# Developing Nocan on Windows

From the repository root in PowerShell:

```powershell
cargo fetch --locked
cargo build --locked
cargo test --lib --locked --offline
```

The build requires a Windows desktop toolchain and resource compiler. Use `rc.exe` with MSVC or `windres` with GNU. The main README describes compiler overrides and runtime requirements.

## Install-and-restart helpers

```powershell
.\tools\run-debug.ps1
.\tools\run-debug.ps1 -Release
```

These commands build offline, stop copies running from this checkout or the managed installation, replace `%LOCALAPPDATA%\Programs\Nocan\nocan.exe`, and launch it again. A failed build leaves the running application alone. Fetch dependencies first and leave `CARGO_TARGET_DIR` unset for these helpers.

`tools/install-nocan.ps1` adds shortcuts and an uninstall entry. `tools/uninstall-nocan.ps1` removes the installed executable and matching shortcuts/startup entry while preserving user data.

For a build that does not replace or stop an existing application, use `cargo build` directly. Close the existing tray instance yourself before launching another normal copy.

## Isolated previews

```powershell
.\target\debug\nocan.exe --preview-branding target\branding-preview
.\target\debug\nocan.exe --dump-settings-preview target\settings-preview
```

Native previews require a Windows graphical session and export raw BGRA frames from the actual renderer. The branding command also checks native click and keyboard paths.

Core preview data uses a temporary directory unless `NOCAN_DATA_DIR` is explicitly set. This does not redirect the shelf, Discord configuration, or third-party applications' local integration directories. See [storage details](data-storage.md) before testing against existing profiles.

Generated raw frames, logs, review images, and build artifacts belong under `target/` and should not be committed. Images deliberately selected for the root README live under `docs/images/`.
