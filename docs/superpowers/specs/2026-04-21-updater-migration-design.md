# Updater Migration Design

Date: 2026-04-21

## Overview

This document defines the migration of `release-hub` from a GitHub-release-centered updater into a general-purpose updater crate whose primary model follows the capability set of `tauri-plugin-updater`, while remaining completely independent from Tauri.

The `updater/` directory currently present in the workspace is reference material only. It must not be added to the repository or treated as a workspace member. Its role is to provide implementation and API guidance for the migration.

## Goals

- Rebuild `release-hub` around a general updater core instead of a GitHub-only flow.
- Align the internal capability model with `tauri-plugin-updater` as closely as practical without carrying over Tauri runtime integration.
- Make endpoint-based update manifests plus minisign signature verification the primary update path.
- Keep GitHub Releases support as a source adapter layered on top of the common updater core.
- Expand platform scope to include Linux in both target resolution and installation design.
- Remove compatibility constraints from the old public API and design the crate around the new model.

## Non-Goals

- Preserve the existing `UpdaterBuilder::new(app_name, version, owner, repo)` API shape.
- Add any Tauri plugin registration, permission model, guest JavaScript bindings, or command IPC APIs.
- Commit the reference `updater/` directory into git.
- Support mobile platforms.
- Implement every Tauri-specific ergonomic detail if it requires runtime concepts that do not exist in a general Rust crate.

## Current State

The current crate centers on GitHub Releases:

- `src/builder.rs` constructs an updater from `owner/repo`.
- `src/github.rs` fetches the latest GitHub release and selects an asset by filename heuristics.
- Installation logic exists for macOS and Windows.
- Linux is not modeled as a first-class installation target.
- There is no manifest-driven update model, no signature verification, and no unified remote release type independent of GitHub.

The reference implementation in `updater/` contains the missing conceptual pieces:

- endpoint-based update manifests
- target resolution
- minisign verification
- a richer builder/config/error model
- Linux installer handling

## Design Summary

`release-hub` will be reorganized into a source-agnostic updater core.

The new default flow is:

1. Build an `Updater` with application metadata, update config, and optional runtime request overrides.
2. Resolve the current target string and installer class.
3. Query a configured source for remote release metadata.
4. Select the release entry for the current target.
5. Compare versions.
6. Download the artifact with progress callbacks.
7. Verify the artifact with minisign using the configured public key.
8. Install the verified artifact using a platform-specific backend.
9. Optionally relaunch the application.

GitHub Releases remain supported, but only as a source adapter that produces the same internal release model used by endpoint manifests.

## Architecture

The crate should be restructured into focused modules with one responsibility each.

### `config`

Defines persistent updater configuration:

- update endpoints
- minisign public key
- transport safety toggles
- Windows-specific install options
- Linux-specific install options and installer strategy hooks

This module owns validation of endpoint safety rules and deserialization of user-provided config.

### `builder`

Defines `UpdaterBuilder` and the final `Updater`.

The builder owns runtime state that should not be embedded in a static config:

- application name
- current version
- executable path override
- target override
- request headers
- timeout
- proxy and no-proxy mode
- installer arguments
- current executable arguments
- custom version comparator
- source selection

### `release`

Defines source-neutral remote update models:

- `ReleaseManifestPlatform`
- `RemoteReleaseInner`
- `RemoteRelease`
- `Update`

This module is the canonical data model for the rest of the crate. Public APIs should expose these models instead of GitHub-specific types.

### `source`

Defines how release metadata is fetched.

This module should introduce a source abstraction, likely a trait such as:

```rust
pub trait ReleaseSource {
    async fn fetch(&self, request: &SourceRequest) -> Result<RemoteRelease>;
}
```

Two initial implementations are required:

- `EndpointSource`
- `GitHubSource`

All downstream logic must operate on `RemoteRelease`, regardless of the original source.

### `target`

Handles target and installer detection:

- OS and architecture resolution
- target string construction
- installer kind detection
- install path derivation

This module must become the single authority for platform selection instead of scattering OS and filename heuristics through the codebase.

### `download`

Handles HTTP client construction and artifact downloads:

- headers
- timeout
- proxy / no-proxy
- progress callbacks
- body buffering

### `verify`

Performs minisign verification of downloaded bytes against the selected platform signature and configured public key.

Verification failure is terminal and must block installation.

### `install`

Contains platform-specific installer backends:

- `macos`
- `windows`
- `linux`

Each backend receives a verified artifact plus resolved install metadata and performs only installation concerns.

### `error`

Defines a source-neutral error model grouped by:

- configuration failures
- manifest and remote-data failures
- network failures
- security failures
- unsupported target failures
- installer and filesystem failures

## Public API

Compatibility with the old API is explicitly out of scope. The public API should be redesigned around the new core model.

### Core Types

```rust
pub struct Config {
    pub endpoints: Vec<Url>,
    pub pubkey: String,
    pub windows: Option<WindowsConfig>,
    pub dangerous_insecure_transport_protocol: bool,
    pub dangerous_accept_invalid_certs: bool,
    pub dangerous_accept_invalid_hostnames: bool,
}

pub struct UpdaterBuilder;
pub struct Updater;

pub struct Update {
    pub current_version: Version,
    pub version: Version,
    pub date: Option<OffsetDateTime>,
    pub body: Option<String>,
    pub raw_json: serde_json::Value,
}
```

### Builder API

The new builder should look conceptually like this:

```rust
UpdaterBuilder::new(app_name, current_version, config)
    .target(...)
    .headers(...)
    .timeout(...)
    .proxy(...)
    .no_proxy()
    .installer_args(...)
    .executable_path(...)
    .version_comparator(...)
    .source(...)
    .build()
```

### Updater API

`Updater` owns update discovery:

- `check() -> Result<Option<Update>>`

It may also expose a convenience:

- `download_and_install(...) -> Result<()>`

### Update API

`Update` owns actions on a specific candidate update:

- `download(...) -> Result<Vec<u8>>`
- `install(bytes: &[u8]) -> Result<()>`
- `download_and_install(...) -> Result<()>`

This separates "discover whether an update exists" from "act on the selected update".

## Source Model

### Endpoint Source

Endpoint manifests are the primary model and must support the official updater manifest structures:

- dynamic single-platform format
- static `platforms[target]` format

Expected release fields include:

- `version`
- `notes`
- `pub_date`
- platform `url`
- platform `signature`

The raw JSON payload should remain accessible on `Update` for advanced consumers.

### GitHub Source

GitHub Releases support remains, but only as an adapter layer.

The adapter must:

1. fetch release metadata from GitHub
2. resolve the artifact for the current target
3. resolve the matching signature asset
4. transform the result into the same `RemoteRelease` model used by endpoint manifests

The adapter must not bypass signature verification. If a matching signature asset cannot be found, the operation should fail with an explicit error rather than silently downgrade security.

GitHub-specific types should not remain the central public abstraction.

## Target Resolution

Target resolution must be generalized beyond the current macOS and Windows-only heuristics.

At minimum, the implementation must support:

- `windows-x86_64`
- `windows-aarch64`
- `darwin-x86_64`
- `darwin-aarch64`
- `linux-x86_64`
- `linux-aarch64`

This module must also determine installer classes:

- Windows: `nsis`, `msi`
- macOS: `.app.tar.gz` preferred, `.app.zip` accepted if retained for compatibility with existing assets
- Linux: `AppImage`, `deb`, `rpm`

Target resolution and installer-class resolution should be testable in isolation.

## Platform Installation Design

### macOS

macOS installation continues to use archive extraction plus atomic bundle replacement:

- unpack the archive
- locate the `.app` bundle
- replace the installed bundle atomically
- request elevation when filesystem permissions require it

This logic can be derived from the current crate and the reference implementation, but it should be separated cleanly from download and source handling.

### Windows

Windows installation continues to stage the installer in a temporary location and launch it with appropriate installer arguments.

The implementation must distinguish at least:

- NSIS
- MSI

The installer backend is responsible for:

- temp-file naming and persistence
- installer argument composition
- elevation path
- stable error reporting for access denied, file in use, cancellation, and execution failure

### Linux

Linux is in scope and must be treated as a first-class platform.

The design should support:

- `AppImage`
- `deb`
- `rpm`

Implementation priority may still differ by installer type, but the architecture must not hard-code Linux as an unsupported afterthought.

Recommended execution strategy:

- `AppImage`: support atomic replacement as the first complete Linux installation path
- `deb` and `rpm`: provide explicit installer flows through system tools or clearly modeled backend hooks

The important design rule is that Linux support must exist in the core target and install model from the start, not as a later structural retrofit.

## Security Model

The migration makes security part of the default flow rather than an optional extension.

Rules:

- HTTPS endpoints are required by default.
- Development-only insecure transport remains opt-in through explicit config.
- Invalid certificates and invalid hostnames are disabled by default and must require explicit opt-in.
- Downloaded artifacts must be verified with minisign before installation.
- GitHub source must not bypass the same verification path.

## Error Model

Errors should be grouped around updater behavior rather than source-specific implementation details.

Suggested categories:

- config validation errors
- version parsing and comparison errors
- remote manifest parse errors
- target resolution errors
- network request and status errors
- signature verification errors
- archive and installer format errors
- temp path and filesystem errors
- permission and elevation errors
- installer execution errors

GitHub-specific errors may still exist internally, but public error semantics should read as updater failures, not as "the crate is really a GitHub wrapper".

## Testing Strategy

Testing should be organized in three layers.

### Unit Tests

- manifest parsing for both supported shapes
- target resolution
- installer-kind detection
- endpoint validation
- GitHub asset and signature pairing
- version comparator behavior

### Backend Tests

- Windows installer argument construction and temp-file handling
- macOS archive and bundle path resolution
- Linux install-command construction and path handling

System decisions should be tested separately from destructive OS actions.

### Integration Tests

Use a local HTTP server to test:

- manifest fetch
- artifact download
- signature verification
- end-to-end `check -> download -> verify`

Installation integration tests should validate that the correct backend path is chosen and that install actions are assembled correctly, while avoiding unsafe mutation of a real user environment.

## Migration Plan

The implementation should proceed in this order:

1. Introduce the new neutral data model and config model.
2. Add source abstraction and endpoint-based update retrieval.
3. Add minisign verification to the common download flow.
4. Refactor target detection into a dedicated module.
5. Migrate platform installers behind backend-specific modules.
6. Reintroduce GitHub Releases as a source adapter on top of the new core.
7. Expand tests to cover manifest, verification, and Linux branches.
8. Rewrite README and crate docs to present endpoint manifests as the primary usage path.

## Open Decisions Already Resolved

The following decisions are fixed for implementation:

- The reference `updater/` directory is for consultation only and must not be committed.
- The crate remains independent from Tauri.
- The new capability model should align with `tauri-plugin-updater` rather than preserving the current simplified API.
- Signature verification and official manifest compatibility are required.
- GitHub Releases support remains as an adapter on top of the common updater core.
- Linux is in scope.
- Backward compatibility with the old public API is not required.

## Acceptance Criteria

The migration is complete when all of the following are true:

- The crate's primary public model is no longer GitHub-specific.
- Endpoint manifests plus minisign verification work end to end.
- GitHub Releases can be consumed through the same internal release and verification flow.
- Target resolution covers macOS, Windows, and Linux.
- Platform installation is structurally separated from source retrieval and download logic.
- The README and crate docs describe the new primary flow.
- The reference `updater/` directory remains untracked and excluded from the repository history.
