use std::io::Read;
use std::path::PathBuf;

pub fn read_input(path: &Option<PathBuf>) -> Result<String, String> {
    match path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("failed to read {p:?}: {e}")),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("failed to read stdin: {e}"))?;
            Ok(buf)
        }
    }
}
