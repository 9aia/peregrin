//! # ac-lib-os
//!
//! Abstract OS service traits for the AC runtime.
//!
//! Defines abstract service interfaces that bridge host capabilities
//! (filesystem, environment, path manipulation, time, command execution, temp files)
//! into callable [`Module`] implementations.
//!
//! ## Architecture
//!
//! ```text
//! ac-runtime-core          ac-lib-os                     ac-lib-os-std
//! ─────────────            ──────────────────            ─────────────────────
//! Module trait             Fs trait                      StdFs (std::fs)
//! ModuleHost               Env trait                     StdEnv (std::env)
//!                          Path trait                    StdPath (std::path)
//!                          Time trait                    StdTime (std::time)
//!                          Cmd trait                     StdCmd (std::process)
//!                          Tmp trait                     StdTmp (std::temp_dir)
//!                          OsError
//!                          *Module adapters
//! ```
//!
//! - **This crate** — abstract service traits + module adapters + test stubs.
//!   No I/O, no platform dependencies.
//! - **`ac-lib-os-std`** — concrete implementations using
//!   Rust stdlib.
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_modules::ModuleHost;
//! use ac_lib_utils::{MathModule, StrModule};
//! use ac_lib_os::{
//!     FsModule, EnvModule, PathModule, TimeModule,
//!     CmdModule, TmpModule,
//!     fs::fs_stub::StubFs,
//!     env::env_stub::StubEnv,
//!     path::path_stub::StubPath,
//!     time::time_stub::StubTime,
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
//!     .with(CmdModule::new(StubCmd::default()))
//!     .with(TmpModule::new(StubTmp::default()));
//! ```
//!
//! Expressions can then call `fs.read("config.yml")`, `env.get("HOME")`,
//! `path.join("a", "b")`, `time.now()`, `cmd.run("echo", "hello")`, `tmp.file()`, etc.

#![allow(clippy::module_inception)]

mod cmd;
mod env;
mod error;
mod fs;
mod path;
mod time;
mod tmp;

pub use error::OsError;

pub use fs::Fs;
pub use fs::FsModule;

pub use env::Env;
pub use env::EnvModule;

pub use path::Path;
pub use path::PathModule;

pub use time::Time;
pub use time::TimeModule;

pub use cmd::Cmd;
pub use cmd::CmdModule;
pub use cmd::CmdOutput;

pub use tmp::Tmp;
pub use tmp::TmpModule;

// Re-export stubs for convenience.
pub use cmd::cmd_stub::StubCmd;
pub use env::env_stub::StubEnv;
pub use fs::fs_stub::StubFs;
pub use path::path_stub::StubPath;
pub use time::time_stub::StubTime;
pub use tmp::tmp_stub::StubTmp;
