use crate::assists::AssistKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistConfig {
    pub allowed: Option<Vec<AssistKind>>,
}
