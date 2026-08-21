//! # ac-lib-platform-std
//!
//! Concrete platform service implementations backed by the Rust stdlib.
//!
//! Provides [`StdFs`] (`std::fs`), [`StdEnv`] (`std::env`),
//! [`StdPath`] (`std::path`), [`StdTime`] (`std::time`),
//! [`StdHttpClient`] (`ureq`), [`StdCmd`] (`std::process`), [`StdTmp`] (`std::temp_dir`).
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_core::ModuleHost;
//! use ac_lib_platform::{FsModule, EnvModule, PathModule, TimeModule, HttpClientModule, CmdModule, TmpModule};
//! use ac_lib_platform_std::{StdFs, StdEnv, StdPath, StdTime, StdHttpClient, StdCmd, StdTmp};
//!
//! let host = ModuleHost::new()
//!     .with(FsModule::new(StdFs))
//!     .with(EnvModule::new(StdEnv))
//!     .with(PathModule::new(StdPath))
//!     .with(TimeModule::new(StdTime))
//!     .with(HttpClientModule::new(StdHttpClient))
//!     .with(CmdModule::new(StdCmd))
//!     .with(TmpModule::new(StdTmp));
//! ```

mod cmd;
mod env;
mod fs;
mod http_client;
mod path;
mod time;
mod tmp;

pub use cmd::StdCmd;
pub use env::StdEnv;
pub use fs::StdFs;
pub use http_client::StdHttpClient;
pub use path::StdPath;
pub use time::StdTime;
pub use tmp::StdTmp;
