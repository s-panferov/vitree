use super::node::{TreeData, TreeKind};
use super::KEY_TYPE;

#[derive(Debug)]
pub struct RootData;

impl TreeData for RootData {
    fn key(&self) -> KEY_TYPE {
        0
    }

    fn icon(&self) -> Option<&str> {
        None
    }

    fn title(&self) -> &str {
        "ROOT"
    }

    fn hash(&self) -> u64 {
        0
    }

    fn expandable(&self) -> bool {
        true
    }
}
