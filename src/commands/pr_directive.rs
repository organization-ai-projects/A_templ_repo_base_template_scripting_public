use std::collections::{HashMap, HashSet};

use crate::regex::{DIRECTIVE_DECISION_REGEX, ISSUE_DIRECTIVE_EVENT_REGEX};

pub(crate) struct ParsedPrDirectives {
    pub(crate) explicit: HashMap<String, String>,
    pub(crate) closing: HashSet<String>,
    pub(crate) reopening: HashSet<String>,
}

pub(crate) struct PrDirective;

impl PrDirective {
    pub(crate) fn parse(payload: &str) -> Result<ParsedPrDirectives, String> {
        let directive_re = DIRECTIVE_DECISION_REGEX
            .as_ref()
            .map_err(|error| format!("Directive regex failed: {error}"))?;
        let event_re = ISSUE_DIRECTIVE_EVENT_REGEX
            .as_ref()
            .map_err(|error| format!("Event regex failed: {error}"))?;

        let mut explicit = HashMap::new();
        let mut closing = HashSet::new();
        let mut reopening = HashSet::new();

        for capture in directive_re.captures_iter(payload) {
            let issue = capture.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let decision = capture
                .get(2)
                .map(|m| m.as_str().to_ascii_lowercase())
                .unwrap_or_default();
            if !issue.is_empty() && !decision.is_empty() {
                explicit.insert(issue, decision);
            }
        }

        for capture in event_re.captures_iter(payload) {
            let action = capture
                .get(1)
                .map(|m| m.as_str().to_ascii_lowercase())
                .unwrap_or_default();
            let rejected = capture.get(2).is_some();
            let issue = capture.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

            if issue.is_empty() {
                continue;
            }

            match action.as_str() {
                "closes" | "fixes" if !rejected => {
                    closing.insert(issue);
                }
                "reopen" | "reopens" if !rejected => {
                    reopening.insert(issue);
                }
                "cancel-closes" => {
                    closing.remove(&issue);
                }
                _ => {}
            }
        }

        Ok(ParsedPrDirectives {
            explicit,
            closing,
            reopening,
        })
    }
}
