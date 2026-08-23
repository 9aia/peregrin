//! # ac-lib-os-std
//!
//! Concrete OS service implementations backed by the Rust stdlib.
//!
//! Provides [`StdFs`] (`std::fs`), [`StdEnv`] (`std::env`),
//! [`StdPath`] (`std::path`), [`StdTime`] (`std::time`),
//! [`StdCmd`] (`std::process`), [`StdTmp`] (`std::temp_dir`).
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_modules::ModuleHost;
//! use ac_lib_os::{FsModule, EnvModule, PathModule, TimeModule, CmdModule, TmpModule};
//! use ac_lib_os_std::{StdFs, StdEnv, StdPath, StdTime, StdCmd, StdTmp};
//!
//! let host = ModuleHost::new()
//!     .with(FsModule::new(StdFs))
//!     .with(EnvModule::new(StdEnv))
//!     .with(PathModule::new(StdPath))
//!     .with(TimeModule::new(StdTime))
//!     .with(CmdModule::new(StdCmd))
//!     .with(TmpModule::new(StdTmp));
//! ```

mod cmd;
mod env;
mod fs;
mod path;
mod time;
mod tmp;

pub use cmd::StdCmd;
pub use env::StdEnv;
pub use fs::StdFs;
pub use path::StdPath;
pub use time::StdTime;
pub use tmp::StdTmp;
