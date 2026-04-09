use regex::Regex;

use crate::commands::{ReferenceInputValue, ReferenceKind, ReferenceNumber};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReferenceInput {
    value: ReferenceInputValue,
    kind: Option<ReferenceKind>,
}

impl ReferenceInput {
    pub(crate) fn from_text(raw: &str) -> Self {
        Self {
            value: ReferenceInputValue::Text(raw.to_string()),
            kind: None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn from_number(value: u64) -> Self {
        Self {
            value: ReferenceInputValue::Number(value),
            kind: None,
        }
    }

    pub(crate) fn with_kind(self, kind: ReferenceKind) -> Self {
        Self {
            value: self.value,
            kind: Some(kind),
        }
    }

    pub(crate) fn with_kind_opt(self, kind: Option<ReferenceKind>) -> Self {
        match kind {
            Some(kind) => self.with_kind(kind),
            None => self,
        }
    }

    pub(crate) fn normalize(&self) -> Option<ReferenceNumber> {
        match &self.value {
            ReferenceInputValue::Text(raw) => ReferenceNumber::parse(raw).map(|reference| {
                self.kind
                    .map(|kind| reference.with_kind(kind))
                    .unwrap_or(reference)
            }),
            ReferenceInputValue::Number(value) => {
                let reference = ReferenceNumber::new(*value);
                Some(
                    self.kind
                        .map(|kind| reference.with_kind(kind))
                        .unwrap_or(reference),
                )
            }
        }
    }

    pub(crate) fn find_in_text(text: &str, kind: Option<ReferenceKind>) -> Vec<ReferenceNumber> {
        let regex = Regex::new(r"(?:^|[^\w])#?(\d+)\b").expect("reference regex must compile");

        regex
            .captures_iter(text)
            .filter_map(|captures| captures.get(1).map(|capture| capture.as_str()))
            .filter_map(|number| ReferenceInput::from_text(number).normalize())
            .map(|reference| match kind {
                Some(kind) => reference.with_kind(kind),
                None => reference,
            })
            .collect()
    }
}
