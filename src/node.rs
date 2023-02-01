use std::cell::{Ref, RefCell, RefMut};
use std::ops::Range;
use std::rc::{Rc, Weak};

use indexmap::IndexMap;

use super::item::TreeItem;
use super::iter::{TreeCursor, TreeNodeIterator};
use super::provider::{TreeExpandResult, TreeProvider};
use super::root::RootData;
use super::{HASH_TYPE, KEY_TYPE};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TreeKind {
    Root,
    Folder,
    File,
}

pub trait TreeData: downcast_rs::Downcast + std::fmt::Debug {
    fn key(&self) -> KEY_TYPE;
    fn title(&self) -> &str;
    fn hash(&self) -> HASH_TYPE;
    fn expandable(&self) -> bool;
}

downcast_rs::impl_downcast!(TreeData);

bitflags::bitflags! {
    pub struct TreeFlags: u32 {
            const ROOT = 0b00000001;
            const EXPANDED = 0b00000010;
            const LOADING = 0b00000100;
            const READY = 0b00001000;
    }
}

#[derive(Debug)]
pub struct TreeNodeInner {
    pub(crate) data: Box<dyn TreeData>,
    depth: u16,
    flags: TreeFlags,
    children_len: usize,
    pub(crate) children: IndexMap<KEY_TYPE, Rc<TreeNode>>,
}

pub struct TreeNode {
    pub(crate) parent: Weak<TreeNode>,
    inner: RefCell<TreeNodeInner>,
}

impl std::fmt::Debug for TreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.borrow().fmt(f)
    }
}

impl TreeNode {
    pub fn new(parent: &Rc<TreeNode>, data: Box<dyn TreeData>) -> Rc<Self> {
        Rc::new(TreeNode {
            inner: RefCell::new(TreeNodeInner {
                data,
                flags: TreeFlags::empty(),
                depth: parent.depth() + 1,
                children: Default::default(),
                children_len: 0,
            }),
            parent: Rc::downgrade(parent),
        })
    }

    pub fn root() -> Rc<Self> {
        Self::root_with_data(Box::new(RootData))
    }

    pub fn root_with_data(data: Box<dyn TreeData>) -> Rc<Self> {
        Rc::new_cyclic(|this| TreeNode {
            inner: RefCell::new(TreeNodeInner {
                data,
                flags: TreeFlags::ROOT,
                children: Default::default(),
                depth: 0,
                children_len: 0,
            }),
            parent: this.clone(),
        })
    }

    pub fn inner(&self) -> Ref<TreeNodeInner> {
        self.inner.borrow()
    }

    pub fn inner_mut(&self) -> RefMut<TreeNodeInner> {
        self.inner.borrow_mut()
    }

    pub fn flags(&self) -> TreeFlags {
        self.inner.borrow_mut().flags
    }

    pub fn set_flags(&self, flags: TreeFlags) {
        self.inner.borrow_mut().flags = flags;
    }

    pub fn get(&self, key: KEY_TYPE) -> Option<Rc<TreeNode>> {
        self.inner_mut().children.get(&key).cloned()
    }

    pub fn title(&self) -> Ref<str> {
        Ref::map(self.inner.borrow(), |v| v.data.title())
    }

    pub fn data(&self) -> Ref<dyn TreeData> {
        Ref::map(self.inner.borrow(), |v| &*v.data)
    }

    pub fn children_len(&self) -> usize {
        self.inner.borrow().children_len
    }

    pub fn first_child(&self) -> Option<Rc<TreeNode>> {
        self.inner().children.first().map(|v| v.1).cloned()
    }

    #[inline(always)]
    pub fn insert(&self, children: Vec<Rc<TreeNode>>) {
        self.insert_inner(children, true);
    }

    fn insert_inner(&self, children: Vec<Rc<TreeNode>>, update_parent: bool) {
        let combined_len = children
            .iter()
            .fold(0, |a, b| a + 1 + b.inner().children_len);

        let mut self_mut = self.inner_mut();

        self_mut.children.extend(children.into_iter().map(|c| {
            let key = { c.inner().data.key() };
            (key, c)
        }));

        self_mut.children_len += combined_len;

        if self_mut.flags.contains(TreeFlags::ROOT) || !update_parent {
            return;
        }

        let mut parent = self.parent.upgrade();
        while let Some(node) = parent {
            node.inner_mut().children_len += combined_len;

            if node.is_root() {
                break;
            }

            parent = node.parent.upgrade();
        }
    }

    pub fn build(
        self: Rc<TreeNode>,
        func: impl FnOnce(&Rc<TreeNode>) -> Vec<Rc<TreeNode>>,
    ) -> Rc<Self> {
        self.insert_inner(func(&self), false);
        self
    }

    pub fn is_root(&self) -> bool {
        self.flags().contains(TreeFlags::ROOT)
    }

    pub fn flatten(self: &Rc<TreeNode>) -> IndexMap<KEY_TYPE, Rc<TreeNode>> {
        let mut list = IndexMap::with_capacity(self.children_len() + 1);
        self.flatten_internal(&mut list);
        list
    }

    // FIXME: we can make this cheaper for CPU if we can cache
    //        unchanged lists
    fn flatten_internal(self: &Rc<TreeNode>, list: &mut IndexMap<KEY_TYPE, Rc<TreeNode>>) {
        if !self.is_root() {
            list.insert(self.key(), self.clone());
        }

        let inner = self.inner();

        if !inner.flags.contains(TreeFlags::EXPANDED) {
            return;
        }

        for child in inner.children.values() {
            child.flatten_internal(list)
        }
    }

    pub(crate) fn find_by_index(self: &Rc<TreeNode>, index: usize) -> Vec<TreeCursor> {
        let mut stack = Vec::new();
        self.find_by_index_internal(index, &mut stack);
        stack
    }

    fn find_by_index_internal(self: &Rc<TreeNode>, index: usize, stack: &mut Vec<TreeCursor>) {
        let mut offset = 0;
        let children = &self.inner().children;
        for child in children.values() {
            let child_inner = child.inner();

            if offset == index {
                stack.push(TreeCursor {
                    offset,
                    node: child.clone(),
                });

                return;
            }

            // next offset
            offset += 1;

            if !child_inner
                .flags
                .contains(TreeFlags::EXPANDED | TreeFlags::LOADING)
            {
                continue;
            }

            if offset + child_inner.children_len >= index {
                return self.find_by_index_internal(index - offset, stack);
            }
        }

        panic!("Index not available")
    }

    // node 1
    // node 2
    // node 3

    // [node1:0, node2:0, node3: 0]

    pub fn slice(self: &Rc<Self>, range: Range<usize>) -> TreeNodeIterator {
        TreeNodeIterator::new(self.clone(), range)
    }
}

// a node can be a provider (for static trees)
impl TreeProvider for TreeNode {
    fn root(&self) -> Rc<TreeNode> {
        if self.is_root() {
            return self.parent.upgrade().unwrap();
        } else {
            panic!()
        }
    }

    fn expand(&self, node: &Rc<TreeNode>) -> TreeExpandResult {
        let mut node = node.inner_mut();
        node.flags.toggle(TreeFlags::EXPANDED);
        TreeExpandResult::Ready
    }
}

impl TreeItem for TreeNode {
    fn key(&self) -> KEY_TYPE {
        self.inner().data.key()
    }

    fn expandable(&self) -> bool {
        self.inner().data.expandable()
    }

    fn title(&self) -> Ref<str> {
        Ref::map(self.inner(), |v| v.data.title())
    }

    fn depth(&self) -> u16 {
        self.inner().depth
    }

    fn hash(&self) -> HASH_TYPE {
        self.inner().data.hash()
    }

    fn expanded(&self) -> bool {
        self.inner().flags.contains(TreeFlags::EXPANDED)
    }
}
