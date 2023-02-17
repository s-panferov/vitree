#![feature(hash_drain_filter)]
#![feature(drain_filter)]

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
