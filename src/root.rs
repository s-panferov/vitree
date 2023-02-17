use crate::node::TreeFlags;

use super::node::TreeData;
use super::KeyType;

#[derive(Debug)]
pub struct RootData;

impl TreeData for RootData {
    fn key(&self) -> KeyType {
        KeyType::MAX
    }

    fn icon(&self) -> Option<&str> {
        None
    }

    fn title(&self) -> &str {
        "ROOT"
    }

    fn hash(&self) -> u64 {
        u64::MAX
    }

    fn flags(&self) -> TreeFlags {
        TreeFlags::EXPANDABLE | TreeFlags::ROOT
    }
}
