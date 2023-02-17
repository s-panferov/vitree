use std::cell::Ref;

use super::{HASH_TYPE, KEY_TYPE};

pub trait TreeItem {
    fn key(&self) -> KEY_TYPE;
    fn icon(&self) -> Ref<str>;
    fn title(&self) -> Ref<str>;
    fn depth(&self) -> u16;
    fn expandable(&self) -> bool {
        false
    }
    fn hash(&self) -> HASH_TYPE;
    fn expanded(&self) -> bool {
        false
    }
}
