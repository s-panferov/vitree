#![feature(hash_extract_if)]
#![feature(extract_if)]

pub mod dynamic;
pub mod item;
pub mod iter;
pub mod node;
pub mod plain;
pub mod provider;
pub mod root;
pub mod view;

pub type KeyType = usize;
pub type HashType = u64;
pub const ROOT_KEY: KeyType = KeyType::MAX;
