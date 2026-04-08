use crate::commands::ReferenceKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReferenceNumber {
    value: u64,
    kind: Option<ReferenceKind>,
}

impl ReferenceNumber {
    pub(crate) fn new(value: u64) -> Self {
        Self { value, kind: None }
    }

    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        let number = trimmed.trim_start_matches('#');

        if number.is_empty() {
            return None;
        }

        number.parse::<u64>().ok().map(Self::new)
    }

    pub(crate) fn with_kind(self, kind: ReferenceKind) -> Self {
        Self {
            value: self.value,
            kind: Some(kind),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn value(&self) -> u64 {
        self.value
    }

    pub(crate) fn as_string(&self) -> String {
        self.value.to_string()
    }

    pub(crate) fn show_number(&self) -> String {
        format!("#{}", self.value)
    }

    pub(crate) fn is_issue(&self) -> bool {
        self.kind == Some(ReferenceKind::Issue)
    }

    pub(crate) fn is_pull_request(&self) -> bool {
        self.kind == Some(ReferenceKind::PullRequest)
    }
}
