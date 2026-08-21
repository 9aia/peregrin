//! # ac-lib-platform
//!
//! Abstract platform service traits for the AC runtime.
//!
//! Inspired by [Effect Platform](https://www.effect.website/docs/v3/platform/introduction),
//! this crate defines abstract service interfaces that bridge host capabilities
//! (filesystem, environment, path manipulation, time, HTTP)
//! into callable [`Module`] implementations.
//!
//! ## Architecture
//!
//! ```text
//! ac-runtime-core          ac-lib-platform               ac-lib-platform-std
//! ─────────────            ──────────────────            ─────────────────────
//! Module trait             Fs trait                      StdFs (std::fs)
//! ModuleHost               Env trait                     StdEnv (std::env)
//!                          Path trait                    StdPath (std::path)
//!                          Time trait                    StdTime (std::time)
//!                          HttpClient trait              StdHttpClient (ureq)
//!                          Cmd trait                     StdCmd (std::process)
//!                          Tmp trait                     StdTmp (std::temp_dir)
//!                          PlatformError
//!                          *Module adapters
//! ```
//!
//! - **This crate** — abstract service traits + module adapters + test stubs.
//!   No I/O, no platform dependencies.
//! - **`ac-lib-platform-std`** — concrete implementations using
//!   Rust stdlib + common crates.
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_utils::{MathModule, StrModule};
//! use ac_lib_platform::{
//!     FsModule, EnvModule, PathModule, TimeModule,
//!     HttpClientModule, CmdModule, TmpModule,
//!     fs::fs_stub::StubFs,
//!     env::env_stub::StubEnv,
//!     path::path_stub::StubPath,
//!     time::time_stub::StubTime,
//!     http_client::http_client_stub::StubHttpClient,
//!     cmd::cmd_stub::StubCmd,
//!     tmp::tmp_stub::StubTmp,
//! };
//!
//! let host = ModuleHost::new()
//!     .with(MathModule)
//!     .with(StrModule)
//!     .with(FsModule::new(StubFs::default()))
//!     .with(EnvModule::new(StubEnv::default()))
//!     .with(PathModule::new(StubPath::default()))
//!     .with(TimeModule::new(StubTime::default()))
//!     .with(HttpClientModule::new(StubHttpClient::default()))
//!     .with(CmdModule::new(StubCmd::default()))
//!     .with(TmpModule::new(StubTmp::default()));
//! ```
//!
//! Expressions can then call `fs.read("config.yml")`, `env.get("HOME")`,
//! `path.join("a", "b")`, `time.now()`, `http.get("https://...")`,
//! `cmd.run("echo", "hello")`, `tmp.file()`, etc.

#![allow(clippy::module_inception)]

mod cmd;
mod env;
mod error;
mod fs;
mod http_client;
mod path;
mod time;
mod tmp;

pub use error::PlatformError;

pub use fs::Fs;
pub use fs::FsModule;

pub use env::Env;
pub use env::EnvModule;

pub use path::Path;
pub use path::PathModule;

pub use time::Time;
pub use time::TimeModule;

pub use http_client::HttpClient;
pub use http_client::HttpClientModule;
pub use http_client::HttpMethod;
pub use http_client::HttpRequest;
pub use http_client::HttpResponse;

pub use cmd::Cmd;
pub use cmd::CmdModule;
pub use cmd::CmdOutput;

pub use tmp::Tmp;
pub use tmp::TmpModule;

// Re-export stubs for convenience.
pub use cmd::cmd_stub::StubCmd;
pub use env::env_stub::StubEnv;
pub use fs::fs_stub::StubFs;
pub use http_client::http_client_stub::StubHttpClient;
pub use path::path_stub::StubPath;
pub use time::time_stub::StubTime;
pub use tmp::tmp_stub::StubTmp;
