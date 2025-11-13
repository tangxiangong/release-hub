<div align=center>
<h1 aligh="center">
ReleaseHub
</h1>
<p align="center">
A simple, cross-platform auto-updater for Rust desktop GUI applications.
</p>
<p align="center">
<a href="https://crates.io/crates/release-hub"> <img alt="Crates.io Version" src="https://img.shields.io/crates/v/release-hub?style=for-the-badge"> </a>
<a href="https://docs.rs/release-hub"> <img alt="docs.rs" src="https://img.shields.io/docsrs/release-hub?style=for-the-badge"> </a>
<img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/crates/l/release-hub?style=for-the-badge">
</p>
</div>


This crate helps your application check for the latest GitHub Releases and download/install the proper artifact for the current platform. It focuses on a minimal API surface, safe defaults, and a predictable end-user experience on macOS and Windows.

## Features

- **GitHub Releases integration** via `octocrab`
- **Semantic versioning** with `semver`
- **Platform-aware asset selection** (macOS .app.zip, Windows .exe/.msi)
- **Progress callback** during download
- **Pluggable headers, proxy and timeout**
- **Atomic install flow** with privilege elevation when required

## Supported platforms

- macOS (unpacks `.app.zip` and swaps the app bundle atomically)
- Windows (downloads `.exe`/`.msi` installer, launches with elevation when needed)

Linux is currently not supported for install flow. The crate compiles on Linux for development, but installation logic is provided only for macOS and Windows.

## Quick start

Add to your Cargo.toml:

```toml
[dependencies]
release-hub = "*"
```

Basic usage:

```rust
use release_hub::{UpdaterBuilder};
use semver::Version;

#[tokio::main]
async fn main() -> release_hub::Result<()> {
    let updater = UpdaterBuilder::new(
        "MyApp",                       // Application name (for temp files / logs)
        "0.1.0",                       // Current version
        "owner",                       // GitHub owner
        "repo",                        // GitHub repo
    )
    .build()?;

    // Option A: one-shot convenience
    let updated = updater.update(|chunk| {
        // chunk = size of bytes received in this tick
        let _ = chunk;
    }).await?;

    if updated {
        // Relaunch when appropriate for your app lifecycle
        // updater.relaunch()?;
    }

    Ok(())
}
```

Manual flow:

```rust
let updater = /* build as above */;
if let Some(ready) = updater.check().await? {
    let bytes = ready.download(|_| {}).await?;
    ready.install(bytes)?;
    // ready.relaunch()?; // Optional: relaunch the updated app
}
```

## Configuration highlights

- **Headers**: attach custom HTTP headers to the download request
- **Proxy**: route download traffic via a proxy (e.g. corporate networks)
- **Timeout**: set a global request timeout
- **Executable path**: override auto-detected path used to compute install target

Example:

```rust
use http::header::{AUTHORIZATION, HeaderValue};
use url::Url;

let updater = UpdaterBuilder::new("MyApp", "0.1.0", "owner", "repo")
    .header(AUTHORIZATION, HeaderValue::from_static("token OAUTH_OR_PAT"))?
    .proxy(Url::parse("http://proxy.local:8080").unwrap())
    .timeout(std::time::Duration::from_secs(60))
    .build()?;
```

## How it works

1. Queries the GitHub API for the latest release
2. Parses assets and picks the correct file for the current OS and CPU arch
3. Downloads the asset with progress callback
4. Installs it:
   - macOS: extracts `.app.zip`, replaces the `.app` atomically, elevating when necessary
   - Windows: writes installer to a temp location and launches it with elevation

## Safety and permissions

- Windows installer is launched with `ShellExecuteW` and `runas` verb for elevation
- macOS uses AppleScript to move bundles when admin privileges are needed
- Operations attempt to be atomic and restore from backup on failure when possible

## FAQ

- Q: Where should I call `relaunch()`?
  A: Only after your UI and background tasks are safely shut down. On Windows, the installer typically handles termination. On macOS, you control when to reopen the app.

- Q: How are assets matched?
  A: Filenames are inspected for OS and arch markers, and known extensions are matched: `.app.zip`, `.dmg`, `.exe`, `.msi`.

## Projects using this crate

- [bibcitex](https://github.com/tangxiangong/bibcitex)
- [fenban](https://github.com/tangxiangong/fenban)

---

## Third-Party Code Attribution

- **Source**: [tauri-apps/tauri-plugin-updater](https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/updater)
- **Author**: The Tauri Programme
- **License**: [MIT](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/LICENSE_MIT) OR [MIT](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/LICENSE_MIT)/[Apache 2.0](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/LICENSE_APACHE-2.0)
- **Usage**: Implement updater for Dioxus apps or any other Rust GUI applications
- **Copyright**:
  ```
  Copyright (c) 2015 - Present - The Tauri Programme within The Commons Conservancy.
  ```
- **Key Modifications**:
  - Adapt for universal Rust crate
  - Remove Tauri-specific runtime integration
  - Use `octocrab` library for GitHub API interaction


> **Detailed Info**: For complete attribution information, please refer to the [**NOTICE**](./NOTICE) file

## License

Licensed under either of:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
