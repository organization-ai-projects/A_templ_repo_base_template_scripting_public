use crate::cli::CliError;
use crate::commands::{ReferenceInput, ReferenceKind, ReferenceNumber};

#[derive(Clone)]
pub(crate) struct CommandArgs {
    pub(crate) args: Vec<String>,
}

impl CommandArgs {
    pub(crate) fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Extracts the value of a specific flag (e.g., --text, --issue).
    pub(crate) fn extract_flag_value(&self, flag: &str) -> Result<String, CliError> {
        let mut i = 0;
        while i < self.args.len() {
            match self.args.get(i).map(String::as_str) {
                Some(f) if f == flag => {
                    i += 1;
                    return self
                        .args
                        .get(i)
                        .cloned()
                        .ok_or(CliError::Usage("flag requires a value"));
                }
                Some(_) => i += 1,
                None => break,
            }
        }
        Err(CliError::Usage("flag is required"))
    }

    pub(crate) fn extract_flag_value_or(&self, flag: &str, default: &str) -> String {
        self.extract_flag_value(flag)
            .unwrap_or_else(|_| default.to_string())
    }

    pub(crate) fn extract_reference_number(
        &self,
        flag: &str,
        kind: Option<ReferenceKind>,
    ) -> Option<ReferenceNumber> {
        self.extract_flag_value(flag)
            .ok()
            .map(|value| ReferenceInput::from_text(&value))
            .map(|reference| match kind {
                Some(kind) => reference.with_kind(kind),
                None => reference,
            })
            .and_then(|reference| reference.normalize())
    }

    pub(crate) fn has_flag(&self, flag: &str) -> bool {
        self.args.iter().any(|arg| arg == flag)
    }

    pub(crate) fn resolve_repo(&self) -> Result<String, String> {
        self.extract_flag_value("--repo")
            .ok()
            .or_else(|| std::env::var("GH_REPO").ok())
            .or_else(|| std::env::var("GITHUB_REPOSITORY").ok())
            .ok_or_else(|| {
                "Missing repository. Use --repo or GH_REPO/GITHUB_REPOSITORY.".to_string()
            })
    }

    pub(crate) fn resolve_pull_request_number(
        &self,
        flag: &str,
    ) -> Result<ReferenceNumber, String> {
        self.extract_reference_number(flag, Some(ReferenceKind::PullRequest))
            .ok_or_else(|| "Invalid or missing pr.".to_string())
    }

    pub(crate) fn resolve_issue_number(&self, flag: &str) -> Result<ReferenceNumber, String> {
        self.extract_reference_number(flag, Some(ReferenceKind::Issue))
            .ok_or_else(|| "Invalid or missing issue.".to_string())
    }
}
