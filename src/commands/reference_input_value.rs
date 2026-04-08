#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ReferenceInputValue {
    Text(String),
    #[allow(dead_code)]
    Number(u64),
}
