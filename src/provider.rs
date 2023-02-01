use std::rc::Rc;

use super::node::TreeNode;

pub enum TreeExpandResult {
	Ready,
	Loading(futures::channel::oneshot::Receiver<Vec<Rc<TreeNode>>>),
}

pub trait TreeProvider {
	fn root(&self) -> Rc<TreeNode>;
	fn expand(&self, node: &Rc<TreeNode>) -> TreeExpandResult;
}
