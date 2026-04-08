use std::collections::HashSet;

use crate::commands::{ReferenceInput, ReferenceKind, ReferenceNumber};

pub(crate) struct ReferenceText;

impl ReferenceText {
    pub(crate) fn build_payload(parts: &[&str]) -> String {
        parts.join("\n")
    }

    pub(crate) fn collect_issue_references<'a, I, F>(lines: I, predicate: F) -> Vec<ReferenceNumber>
    where
        I: IntoIterator<Item = &'a str>,
        F: Fn(&str) -> bool,
    {
        let mut seen = HashSet::new();
        let mut references = Vec::new();

        for line in lines {
            if !predicate(line) {
                continue;
            }

            for reference in ReferenceInput::find_in_text(line, Some(ReferenceKind::Issue)) {
                let key = reference.as_string();
                if seen.insert(key) {
                    references.push(reference);
                }
            }
        }

        references
    }
}
