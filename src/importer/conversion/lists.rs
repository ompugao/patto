use super::Conversion;
use crate::parser::AstNode;

/// Nesting of the lists currently open.
#[derive(Default)]
pub(super) struct Lists {
    /// One entry per open list, `true` when that list is ordered.
    pub(super) stack: Vec<bool>,
    /// Line the outermost list hangs from.
    pub(super) root: Option<AstNode>,
    /// Depth of the item being built, i.e. the stack size when it started.
    pub(super) depth: usize,
    /// `Some` while building a task list item, holding its checkbox state.
    pub(super) task_checked: Option<bool>,
}

impl Conversion<'_> {
    pub(super) fn start_list(&mut self, ordered: bool) {
        // A nested list interrupts its parent item, so close that item's line first.
        if let Some(line_node) = self.line_node.take() {
            self.flush_pending_into(&line_node);
            if self.lists.task_checked.take().is_some() {
                self.report.statistics.increment_feature("tasks");
            }
            self.attach_list_item(line_node);
        }

        if self.lists.stack.is_empty() {
            let list_root = AstNode::line("", self.line, None, None);
            self.root.add_child(list_root.clone());
            self.lists.root = Some(list_root);
        }

        self.lists.stack.push(ordered);
        self.report.statistics.increment_feature("lists");
    }

    pub(super) fn end_list(&mut self) {
        self.lists.stack.pop();
        self.lists.depth = self.lists.stack.len();
        if self.lists.stack.is_empty() {
            self.lists.root = None;
        }
    }

    pub(super) fn start_item(&mut self) {
        self.lists.depth = self.lists.stack.len();
        self.lists.task_checked = None;
        self.line_node = Some(AstNode::line("", self.line, None, None));
    }

    pub(super) fn end_item(&mut self) {
        let checked = self.lists.task_checked.take();
        let properties = checked.map(|checked| self.task_property(checked));

        let line_node = AstNode::line("", self.line, None, properties);
        self.flush_pending_into(&line_node);

        // The line created on Tag::Item was only a placeholder.
        self.line_node = None;
        self.attach_list_item(line_node);
    }

    fn attach_list_item(&self, line_node: AstNode) {
        let parent = self.lists.root.as_ref().unwrap_or(&self.root);
        add_child_at_depth(parent, line_node, self.lists.depth);
    }
}

/// Attach `child` under the item chain of `root`, `depth` levels down.
/// `depth <= 1` makes it a direct child.
fn add_child_at_depth(root: &AstNode, child: AstNode, depth: usize) {
    if depth <= 1 {
        root.add_child(child);
        return;
    }
    add_child_at_depth_recursive(root, child, depth - 1);
}

fn add_child_at_depth_recursive(node: &AstNode, child: AstNode, remaining_depth: usize) {
    let children = node.children();
    let Some(last_child) = children.last().cloned() else {
        drop(children);
        node.add_child(child);
        return;
    };
    drop(children);

    if remaining_depth == 1 {
        last_child.add_child(child);
    } else {
        add_child_at_depth_recursive(&last_child, child, remaining_depth - 1);
    }
}
