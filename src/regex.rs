use regex::Regex;
use std::sync::LazyLock;

pub(crate) static ISSUE_REF_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(closes|fixes)\b\s+(rejected\s+)?[^#\s]*#([0-9]+)"));

pub(crate) static REJECTED_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(closes|fixes)\s+rejected\s+(#[0-9]+)\b"));

pub(crate) static CHECKBOX_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"- \[[ xX]\]"));

pub(crate) static DIRECTIVE_DECISION_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?im)^Directive Decision:\s*(#\d+)\s*=>\s*(close|reopen)\s*$"));

pub(crate) static ISSUE_DIRECTIVE_EVENT_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| {
        Regex::new(r"(?i)\b(cancel-closes|closes|fixes|reopen|reopens)\b\s+(rejected\s+)?(#\d+)")
    });
