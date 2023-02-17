use std::cell::Ref;

use super::{HashType, KeyType};

pub trait TreeItem {
    fn key(&self) -> KeyType;
    fn icon(&self) -> Ref<str>;
    fn title(&self) -> Ref<str>;
    fn depth(&self) -> u16;
    fn expandable(&self) -> bool {
        false
    }
    fn hash(&self) -> HashType;
    fn expanded(&self) -> bool {
        false
    }
}
