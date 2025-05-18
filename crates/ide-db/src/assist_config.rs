use crate::AllowSnippets;
use crate::assists::AssistKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistConfig {
    pub snippet_cap: Option<AllowSnippets>,
    pub allowed: Option<Vec<AssistKind>>,
    pub code_action_grouping: bool,
}
