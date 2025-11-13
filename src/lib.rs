#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

// Copyright (c) 2015 - Present - The Tauri Programme within The Commons Conservancy.
//
// License: Apache-2.0 OR MIT/Apache-2.0
//
// Modified by tangxiangong (2025) for [release-hub](https://github.com/tangxiangong/release-hub).
//
// # Note
//
// This crate is forked and modified from the [tauri-apps/tauri-plugin-updater](https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/updater), which is licensed under [MIT](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/LICENSE_MIT) or [Apache 2.0](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/LICENSE_APACHE-2.0)/[MIT](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/LICENSE_MIT).

mod builder;
pub use builder::*;
mod error;
pub use error::*;
pub mod github;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
pub use github::*;
pub mod utils;
pub use utils::*;
