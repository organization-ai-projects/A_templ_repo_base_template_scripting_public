use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::commands::CommandArgs;

pub(crate) struct ScriptsIntegrity {
    args: CommandArgs,
}

impl ScriptsIntegrity {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self { args }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let worktree = self.args.extract_flag_value_or("--worktree", ".");
        let workflow_roots = [
            Path::new(&worktree).join("repository_elements/.github/workflows"),
            Path::new(&worktree).join(".github/workflows"),
        ];
        let mut workflow_files = Vec::new();

        for root in workflow_roots {
            if root.exists() {
                workflow_files.extend(self.collect_workflow_files(&root)?);
            }
        }

        if workflow_files.is_empty() {
            return Err("No workflow templates found to validate.".to_string());
        }

        let forbidden_command_pattern =
            Regex::new(r"\b(bash|sh|python|python3|perl|corepack|jq|sed|awk|grep)\b")
                .map_err(|error| format!("Invalid workflow integrity regex: {error}"))?;

        let mut failures = Vec::new();

        for workflow_file in workflow_files {
            let content = fs::read_to_string(&workflow_file).map_err(|error| {
                format!(
                    "Failed to read workflow file '{}': {error}",
                    workflow_file.display()
                )
            })?;

            for (index, raw_line) in content.lines().enumerate() {
                let line_number = index + 1;
                let trimmed = raw_line.trim();

                if trimmed.starts_with("run: |") {
                    failures.push(format!(
                        "{}:{} uses multiline shell run blocks",
                        workflow_file.display(),
                        line_number
                    ));
                }

                if trimmed.contains("scripts/automation/") {
                    failures.push(format!(
                        "{}:{} still references scripts/automation",
                        workflow_file.display(),
                        line_number
                    ));
                }

                if let Some(command) = trimmed.strip_prefix("run:") {
                    let command = command.trim();
                    if !command.is_empty() && !command.starts_with("cargo ") {
                        failures.push(format!(
                            "{}:{} uses non-Rust run command '{}'",
                            workflow_file.display(),
                            line_number,
                            command
                        ));
                    }
                }

                if forbidden_command_pattern.is_match(trimmed) {
                    failures.push(format!(
                        "{}:{} contains forbidden shell tooling '{}'",
                        workflow_file.display(),
                        line_number,
                        trimmed
                    ));
                }
            }
        }

        if failures.is_empty() {
            println!("Workflow integrity checks passed.");
            Ok(())
        } else {
            Err(format!(
                "Workflow integrity checks failed:\n{}",
                failures.join("\n")
            ))
        }
    }

    fn collect_workflow_files(&self, root: &Path) -> Result<Vec<PathBuf>, String> {
        let mut files = Vec::new();
        self.collect_workflow_files_recursive(root, &mut files)?;
        Ok(files)
    }

    fn collect_workflow_files_recursive(
        &self,
        root: &Path,
        files: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        for entry in fs::read_dir(root)
            .map_err(|error| format!("Failed to read directory '{}': {error}", root.display()))?
        {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to read directory entry in '{}': {error}",
                    root.display()
                )
            })?;
            let path = entry.path();

            if path.is_dir() {
                self.collect_workflow_files_recursive(&path, files)?;
                continue;
            }

            let is_yaml = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| matches!(extension, "yml" | "yaml"))
                .unwrap_or(false);

            if is_yaml {
                files.push(path);
            }
        }

        Ok(())
    }
}
