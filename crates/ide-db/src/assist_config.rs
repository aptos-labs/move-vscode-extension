use crate::SnippetCap;
use crate::assists::AssistKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistConfig {
    pub snippet_cap: Option<SnippetCap>,
    pub allowed: Option<Vec<AssistKind>>,
    pub code_action_grouping: bool,
}
