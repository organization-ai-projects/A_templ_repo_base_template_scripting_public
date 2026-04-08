use std::env;
use std::fs;

use regex::Regex;

use crate::commands::{CommandArgs, GitHubActions};

pub(crate) struct WorkflowRunPr {
    pub(crate) args: CommandArgs,
    github_actions: GitHubActions,
}

impl WorkflowRunPr {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github_actions: GitHubActions,
        }
    }

    pub(crate) fn resolve_to_github_output(&self) -> Result<(), String> {
        let event_path = self.resolve_event_path()?;
        let event_payload = fs::read_to_string(&event_path)
            .map_err(|error| format!("Failed to read event payload '{event_path}': {error}"))?;
        let pr_number = self.extract_pr_number(&event_payload);

        match pr_number {
            Some(pr_number) => {
                self.github_actions
                    .write_output("pr_number", &pr_number.to_string())?;
            }
            None => {
                println!("No pull request associated with this workflow run; skipping.");
                self.github_actions.write_output("pr_number", "")?;
            }
        }

        Ok(())
    }

    fn resolve_event_path(&self) -> Result<String, String> {
        self.args
            .extract_flag_value("--event-path")
            .ok()
            .or_else(|| env::var("GITHUB_EVENT_PATH").ok())
            .ok_or_else(|| "Missing --event-path and GITHUB_EVENT_PATH".to_string())
    }

    fn extract_pr_number(&self, event_payload: &str) -> Option<u64> {
        let regex =
            Regex::new(r#"(?s)"pull_requests"\s*:\s*\[\s*\{.*?"number"\s*:\s*([0-9]+)"#).ok()?;

        regex
            .captures(event_payload)
            .and_then(|captures| captures.get(1))
            .and_then(|number| number.as_str().parse::<u64>().ok())
    }
}
