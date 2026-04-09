use crate::commands::breaking_change_analysis::BreakingChangeAnalysis;

pub(crate) struct ValidationGateState {
    breaking_change: BreakingChangeAnalysis,
}

impl ValidationGateState {
    pub(crate) fn new(breaking_change: BreakingChangeAnalysis) -> Self {
        Self { breaking_change }
    }

    pub(crate) fn render_section(&self) -> String {
        let mut lines = vec!["### Validation Gate".to_string(), String::new()];

        if self.breaking_change.is_breaking() {
            lines.push("- Breaking change".to_string());
            lines.push("- Breaking scope:".to_string());
            lines.push(format!(
                "  - crate(s): {}",
                if self.breaking_change.scopes().is_empty() {
                    "unknown".to_string()
                } else {
                    self.breaking_change
                        .scopes()
                        .iter()
                        .map(|scope| format!("`{scope}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ));
            lines.push(format!(
                "  - source commit(s): {}",
                if self.breaking_change.commits().is_empty() {
                    "unknown".to_string()
                } else {
                    self.breaking_change
                        .commits()
                        .iter()
                        .map(|commit| format!("`{commit}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ));
        } else {
            lines.push("- No breaking change".to_string());
        }

        lines.join("\n")
    }

    pub(crate) fn replace_in_body(&self, body: &str) -> String {
        let section = self.render_section();
        let issue_outcomes_marker = "\n### Issue Outcomes\n";

        if let Some(start) = body.find("### Validation Gate\n")
            && let Some(relative_end) = body[start..].find(issue_outcomes_marker)
        {
            let end = start + relative_end;
            let mut updated = String::new();
            updated.push_str(&body[..start]);
            updated.push_str(&section);
            updated.push_str(issue_outcomes_marker);
            updated.push_str(&body[end + issue_outcomes_marker.len()..]);
            return updated;
        }

        let base = body.trim_end_matches('\n');
        if base.is_empty() {
            section
        } else {
            format!("{base}\n\n{section}")
        }
    }
}
