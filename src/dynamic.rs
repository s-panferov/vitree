use std::cell::RefCell;
use std::rc::{Rc, Weak};

use indexmap::IndexMap;
use skima::web::Callback;
use wasm_bindgen::UnwrapThrowExt;

use super::item::TreeItem;
use super::node::{TreeFlags, TreeNode};
use super::provider::{TreeExpandResult, TreeProvider};
use super::view::TreeController;
use super::KeyType;

pub trait TreeSubscriber {
    // Update the whole tree
    fn update_all(&self);

    // Update single item
    fn update_item(&self, key: usize);
}

pub struct DynamicTree {
    this: Weak<Self>,
    root: Rc<TreeNode>,
    callbacks: TreeCallbacks,
    flat: RefCell<IndexMap<usize, Rc<TreeNode>>>,
    provider: Rc<dyn TreeProvider>,
    subscribers: RefCell<Vec<Weak<dyn TreeSubscriber>>>,
}

#[derive(Default)]
pub struct TreeCallbacks {
    pub on_click: Option<Callback<dyn Fn(Rc<TreeNode>)>>,
}

impl DynamicTree {
    pub fn new(provider: Rc<dyn TreeProvider>, callbacks: TreeCallbacks) -> Rc<Self> {
        let root = provider.root();
        let flat = root.flatten();

        Rc::new_cyclic(|this| DynamicTree {
            this: this.clone(),
            root,
            callbacks,
            provider,
            subscribers: Default::default(),
            flat: RefCell::new(flat),
        })
    }

    pub fn root(&self) -> Rc<TreeNode> {
        self.root.clone()
    }

    pub fn flatten(&self) {
        self.flat.replace(self.root.flatten());
    }

    fn for_each_subscriber(&self, func: impl Fn(&dyn TreeSubscriber)) {
        self.subscribers
            .borrow_mut()
            .extract_if(|c| {
                if let Some(c) = c.upgrade() {
                    func(&*c);
                    false
                } else {
                    true
                }
            })
            .for_each(|_| {});
    }

    fn notify_update_all(&self) {
        self.for_each_subscriber(|c| c.update_all())
    }

    fn notify_update_item(&self, key: KeyType) {
        self.for_each_subscriber(|c| c.update_item(key))
    }

    fn get_item(&self, key: usize) -> Rc<TreeNode> {
        if self.root.key() == key {
            self.root.clone()
        } else {
            let flat = self.flat.borrow();
            let item = flat.get(&key).unwrap();
            item.clone()
        }
    }

    pub fn expand(&self, key: KeyType) {
        let item = self.get_item(key);
        let mut flags = item.flags();

        if !flags.contains(TreeFlags::EXPANDABLE) {
            return;
        }

        if flags.contains(TreeFlags::EXPANDED) {
            // Collapse
            flags.remove(TreeFlags::EXPANDED);
            item.set_flags(flags);
            self.flat.replace(self.root.flatten());
            self.notify_update_all();
            return;
        }

        if flags.contains(TreeFlags::READY) {
            // Just expand
            flags.insert(TreeFlags::EXPANDED);
            item.set_flags(flags);
            self.flat.replace(self.root.flatten());
            self.notify_update_all();

            return;
        }

        match self.provider.expand(&item) {
            TreeExpandResult::Ready => {
                let mut flags = item.flags();

                flags.insert(TreeFlags::EXPANDED);
                flags.insert(TreeFlags::READY);
                item.set_flags(flags);

                self.flat.replace(self.root.flatten());
                self.notify_update_all();
            }
            TreeExpandResult::Async(job) => {
                let mut flags = item.flags();

                flags.toggle(TreeFlags::LOADING);

                item.set_flags(flags);
                self.notify_update_item(key);
                let this = self.this.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let result = job.await.unwrap_throw();

                    flags.remove(TreeFlags::LOADING);
                    flags.insert(TreeFlags::EXPANDED);
                    flags.insert(TreeFlags::READY);
                    item.set_flags(flags);

                    item.insert(result);

                    if let Some(this) = this.upgrade() {
                        this.flat.replace(this.root.flatten());
                        tracing::info!("{:?}", this.root.flatten());
                        this.notify_update_all();
                        tracing::info!("Expanded");
                    }
                })
            }
        }
    }

    fn on_click(&self, item: Rc<TreeNode>) {
        if let Some(on_click) = self.callbacks.on_click.as_ref() {
            on_click(item)
        }
    }
}

impl TreeController for DynamicTree {
    fn item(&self, index: usize) -> Rc<dyn TreeItem> {
        self.flat.borrow().get_index(index).unwrap().1.clone()
    }

    fn handle_click(&self, key: KeyType) {
        self.expand(key);

        let item = self.get_item(key);
        self.on_click(item);
    }

    fn count(&self) -> usize {
        self.flat.borrow().len()
    }

    fn add_subscriber(&self, subscriber: Rc<dyn TreeSubscriber>) {
        self.subscribers
            .borrow_mut()
            .push(Rc::downgrade(&subscriber))
    }
}
