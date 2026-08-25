use std::io::Read;
use std::path::PathBuf;

pub fn read_inputs(paths: &[PathBuf]) -> Result<Vec<(Option<PathBuf>, String)>, String> {
    if paths.is_empty() {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        Ok(vec![(None, buf)])
    } else {
        paths
            .iter()
            .map(|p| {
                std::fs::read_to_string(p)
                    .map(|s| (Some(p.clone()), s))
                    .map_err(|e| format!("failed to read {p:?}: {e}"))
            })
            .collect()
    }
}
