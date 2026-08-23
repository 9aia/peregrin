use ac_lib_os::{OsError, Sig};

/// Signal implementation backed by platform signal APIs.
#[derive(Default)]
pub struct StdSig;

#[cfg(unix)]
fn parse_signal(signal: &str) -> Result<nix::sys::signal::Signal, OsError> {
    use nix::sys::signal::Signal;
    match signal {
        "SIGINT" => Ok(Signal::SIGINT),
        "SIGTERM" => Ok(Signal::SIGTERM),
        "SIGKILL" => Ok(Signal::SIGKILL),
        "SIGHUP" => Ok(Signal::SIGHUP),
        "SIGUSR1" => Ok(Signal::SIGUSR1),
        "SIGUSR2" => Ok(Signal::SIGUSR2),
        "SIGCHLD" => Ok(Signal::SIGCHLD),
        "SIGCONT" => Ok(Signal::SIGCONT),
        "SIGSTOP" => Ok(Signal::SIGSTOP),
        "SIGTSTP" => Ok(Signal::SIGTSTP),
        _ => Err(OsError::InvalidArgument {
            message: format!("unknown signal: {signal}")})}
}

impl Sig for StdSig {
    fn send(&self, process_id: i64, signal: &str) -> Result<(), OsError> {
        #[cfg(unix)]
        {
            let sig = parse_signal(signal)?;
            let pid = nix::unistd::Pid::from_raw(process_id as i32);
            nix::sys::signal::kill(pid, sig).map_err(|e| OsError::Io {
                message: e.to_string()})
        }
        #[cfg(not(unix))]
        Err(OsError::InvalidArgument {
            message: "sending signals is not supported on this platform".into()})
    }

    fn list(&self) -> Result<Vec<String>, OsError> {
        #[cfg(unix)]
        {
            Ok(vec![
                "SIGINT".into(),
                "SIGTERM".into(),
                "SIGKILL".into(),
                "SIGHUP".into(),
                "SIGUSR1".into(),
                "SIGUSR2".into(),
                "SIGCHLD".into(),
                "SIGCONT".into(),
                "SIGSTOP".into(),
                "SIGTSTP".into()])
        }
        #[cfg(not(unix))]
        Ok(Vec::new())
    }
}
