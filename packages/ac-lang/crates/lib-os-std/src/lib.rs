//! # ac-lib-os-std
//!
//! Concrete OS service implementations backed by the Rust stdlib.
//!
//! Provides [`StdFs`] (`std::fs`), [`StdEnv`] (`std::env`),
//! [`StdPath`] (`std::path`), [`StdTime`] (`std::time`),
//! [`StdCmd`] (`std::process`), [`StdTmp`] (`std::temp_dir`),
//! [`StdSocket`] (`std::net`), [`StdSig`] (platform signals).
//!
//! ## Usage
//!
//! ```ignore
//! use ac_runtime_modules::ModuleHost;
//! use ac_lib_os::{FsModule, EnvModule, PathModule, TimeModule, CmdModule, TmpModule, SocketModule, SigModule};
//! use ac_lib_os_std::{StdFs, StdEnv, StdPath, StdTime, StdCmd, StdTmp, StdSocket, StdSig};
//!
//! let host = ModuleHost::new()
//!     .with(FsModule::new(StdFs))
//!     .with(EnvModule::new(StdEnv))
//!     .with(PathModule::new(StdPath))
//!     .with(TimeModule::new(StdTime))
//!     .with(CmdModule::new(StdCmd))
//!     .with(TmpModule::new(StdTmp))
//!     .with(SocketModule::new(StdSocket))
//!     .with(SigModule::new(StdSig));
//! ```

mod cmd;
mod env;
mod fs;
mod path;
mod sig;
mod socket;
mod time;
mod tmp;

#[cfg(unix)]
mod mio_socket;

#[cfg(test)]
mod sig_tests;
#[cfg(test)]
mod socket_tests;
#[cfg(all(test, unix))]
mod mio_socket_tests;

pub use cmd::StdCmd;
pub use env::StdEnv;
pub use fs::StdFs;
pub use path::StdPath;
pub use sig::StdSig;
pub use socket::StdSocket;
#[cfg(unix)]
pub use mio_socket::MioSocket;
pub use time::StdTime;
pub use tmp::StdTmp;
