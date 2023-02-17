use std::borrow::Cow;

use super::node::{TreeData, TreeKind};
use super::{HASH_TYPE, KEY_TYPE};

#[derive(Debug)]
pub struct PlainTreeData {
    pub key: KEY_TYPE,
    pub icon: Option<Cow<'static, str>>,
    pub title: String,
    pub expandable: bool,
}

impl TreeData for PlainTreeData {
    fn key(&self) -> KEY_TYPE {
        self.key
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_ref().map(|s| s.as_ref())
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn expandable(&self) -> bool {
        self.expandable
    }

    fn hash(&self) -> HASH_TYPE {
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
                        expandable: true,
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
                                expandable: false,
                            }),
                        ),
                        TreeNode::new(
                            parent,
                            Box::new(PlainTreeData {
                                key: 12,
                                icon: None,
                                title: "1.2".into(),
                                expandable: false,
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
                        expandable: false,
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
