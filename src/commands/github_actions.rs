use std::env;
use std::fs::OpenOptions;
use std::io::Write;

pub(crate) struct GitHubActions;

impl GitHubActions {
    pub(crate) fn write_output(&self, key: &str, value: &str) -> Result<(), String> {
        let output_path =
            env::var("GITHUB_OUTPUT").map_err(|_| "Missing GITHUB_OUTPUT".to_string())?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&output_path)
            .map_err(|error| format!("Failed to open GITHUB_OUTPUT at '{output_path}': {error}"))?;
        writeln!(file, "{key}={value}")
            .map_err(|error| format!("Failed to write {key} to GITHUB_OUTPUT: {error}"))?;
        Ok(())
    }
}
