use std::borrow::Cow;

use crate::node::TreeFlags;

use super::node::{TreeData, TreeKind};
use super::{HashType, KeyType};

#[derive(Debug)]
pub struct PlainTreeData {
    pub key: KeyType,
    pub icon: Option<Cow<'static, str>>,
    pub title: String,
    pub flags: TreeFlags,
}

impl TreeData for PlainTreeData {
    fn key(&self) -> KeyType {
        self.key
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_ref().map(|s| s.as_ref())
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn flags(&self) -> TreeFlags {
        self.flags
    }

    fn hash(&self) -> HashType {
        fxhash::hash64(&(&self.key, &self.title))
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use crate::node::TreeNode;

    #[test]
    fn test() {
        let tree = TreeNode::root().build(|parent| {
            vec![
                TreeNode::new(
                    parent,
                    Box::new(PlainTreeData {
                        key: 1,
                        icon: None,
                        title: "1".into(),
                        flags: TreeFlags::EXPANDABLE,
                    }),
                )
                .build(|parent| {
                    vec![
                        TreeNode::new(
                            parent,
                            Box::new(PlainTreeData {
                                key: 11,
                                icon: None,
                                title: "1.1".into(),
                                flags: TreeFlags::empty(),
                            }),
                        ),
                        TreeNode::new(
                            parent,
                            Box::new(PlainTreeData {
                                key: 12,
                                icon: None,
                                title: "1.2".into(),
                                flags: TreeFlags::empty(),
                            }),
                        ),
                    ]
                }),
                TreeNode::new(
                    parent,
                    Box::new(PlainTreeData {
                        key: 2,
                        icon: None,
                        title: "2".into(),
                        flags: TreeFlags::empty(),
                    }),
                ),
            ]
        });

        assert_eq!(tree.children_len(), 4);

        let stack = tree.flatten();

        println!("Tree {:#?}", stack.len());

        // let mut item = tree.slice(0..100);
        // assert_eq!(*item.next().unwrap().name(), "1");
        // assert_eq!(*item.next().unwrap().name(), "1.1");
        // assert_eq!(*item.next().unwrap().name(), "1.2");
        // assert_eq!(*item.next().unwrap().name(), "2");
    }
}
