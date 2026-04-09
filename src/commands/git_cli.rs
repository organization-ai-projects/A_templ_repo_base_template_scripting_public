use std::process::Command;

pub(crate) struct GitCli;

impl GitCli {
    pub(crate) fn log_commit_records(
        &self,
        worktree: &str,
        range: &str,
    ) -> Result<Vec<(String, String, String)>, String> {
        let output = Command::new("git")
            .args(["log", "--format=%H%x1f%s%x1f%b%x1e", range])
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("Failed to execute git log: {error}"))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .split('\x1e')
            .filter_map(|record| {
                if record.trim().is_empty() {
                    return None;
                }
                let mut parts = record.splitn(3, '\x1f');
                let hash = parts.next()?.trim().to_string();
                let subject = parts.next().unwrap_or_default().trim().to_string();
                let body = parts.next().unwrap_or_default().trim().to_string();
                Some((hash, subject, body))
            })
            .collect())
    }

    pub(crate) fn fetch_branches(
        &self,
        worktree: &str,
        base_ref: &str,
        head_ref: &str,
    ) -> Result<(), String> {
        let output = Command::new("git")
            .args([
                "fetch",
                "--no-tags",
                "--depth=1",
                "origin",
                base_ref,
                head_ref,
            ])
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("Failed to execute git fetch: {error}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "git fetch failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    pub(crate) fn diff_name_only(
        &self,
        worktree: &str,
        range: &str,
    ) -> Result<Vec<String>, String> {
        let output = Command::new("git")
            .args(["diff", "--name-only", range])
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("Failed to execute git diff: {error}"))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect())
    }

    pub(crate) fn diff_name_only_with_filter(
        &self,
        worktree: &str,
        range: &str,
        diff_filter: &str,
    ) -> Result<Vec<String>, String> {
        let output = Command::new("git")
            .args([
                "diff",
                "--name-only",
                &format!("--diff-filter={diff_filter}"),
                range,
            ])
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("Failed to execute git diff: {error}"))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect())
    }

    pub(crate) fn has_diff(&self, worktree: &str) -> Result<bool, String> {
        let output = Command::new("git")
            .args(["diff", "--quiet"])
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("Failed to execute git diff --quiet: {error}"))?;

        Ok(!output.status.success())
    }

    pub(crate) fn rustfmt_files(
        &self,
        worktree: &str,
        files: &[String],
        check_only: bool,
    ) -> Result<bool, String> {
        if files.is_empty() {
            return Ok(true);
        }

        let mut args = vec!["--edition", "2024"];
        if check_only {
            args.push("--check");
        }
        for file in files {
            args.push(file);
        }

        let output = Command::new("rustfmt")
            .args(args)
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("Failed to execute rustfmt: {error}"))?;

        Ok(output.status.success())
    }
}
