use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum CliError {
    Usage(&'static str),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => f.write_str(message),
        }
    }
}
