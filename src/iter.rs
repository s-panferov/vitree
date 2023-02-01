use std::ops::Range;
use std::rc::Rc;

use super::item::TreeItem;
use super::node::TreeNode;

pub struct TreeNodeIterator {
	range: Range<usize>,
	stack: Vec<TreeCursor>,
}

impl TreeNodeIterator {
	pub fn new(root: Rc<TreeNode>, range: Range<usize>) -> Self {
		let stack = root.find_by_index(range.start);
		TreeNodeIterator { range, stack }
	}

	pub fn next(&mut self) -> Option<Rc<TreeNode>> {
		let Some(frame) = self.stack.last().cloned() else { return None };

		// first check if we can go down to children
		if frame.node.expanded() {
			if let Some(child) = frame.node.first_child() {
				self.stack.push(TreeCursor {
					node: child,
					offset: 0,
				});

				return Some(frame.node);
			}
		}

		// then check if we have a next sibling
		if let Some(next_sibling) = frame.next_sibling() {
			self.stack.push(next_sibling);
			return Some(frame.node);
		}

		// remove the frame from the stack
		self.stack.pop().unwrap();

		while let Some(parent) = self.stack.last_mut() {
			if let Some(sibling) = parent.next_sibling() {
				*parent = sibling;
				return Some(frame.node);
			}

			self.stack.pop();
		}

		Some(frame.node)
	}
}

#[derive(Clone)]
pub struct TreeCursor {
	pub node: Rc<TreeNode>,
	pub offset: usize,
}

impl TreeCursor {
	pub fn next_sibling(&self) -> Option<TreeCursor> {
		let parent = self.node.parent.upgrade().unwrap();
		let next_offset = self.offset + 1;

		if let Some(next_sibling) = parent
			.inner()
			.children
			.get_index(next_offset)
			.map(|v| v.1)
			.cloned()
		{
			Some(TreeCursor {
				node: next_sibling,
				offset: next_offset,
			});
		}

		None
	}
}
