use super::Conversion;
use crate::parser::AstNode;

/// Nesting of the lists currently open.
#[derive(Default)]
pub(super) struct Lists {
    /// Number of lists open around the current position.
    open: usize,
    /// Line the outermost list hangs from.
    root: Option<AstNode>,
    /// One entry per open item, innermost last.
    items: Vec<ListItem>,
}

/// A list item between its start and end events.
struct ListItem {
    /// Lists open when the item started, which is its depth under the list root.
    depth: usize,
    /// `Some` for a task list item, holding its checkbox state.
    task_checked: Option<bool>,
    /// The item's line, once written. A nested list or a paragraph break
    /// forces it out before the item ends; whatever follows hangs under it.
    line: Option<AstNode>,
}

impl Conversion<'_> {
    pub(super) fn start_list(&mut self) {
        // A nested list interrupts its parent item: write the parent's line
        // now so the nested items have something to hang from.
        self.write_item_line();

        if self.lists.open == 0 {
            let list_root = AstNode::line("", self.line, None, None);
            self.root.add_child(list_root.clone());
            self.lists.root = Some(list_root);
        }

        self.lists.open += 1;
        self.report.statistics.increment_feature("lists");
    }

    pub(super) fn end_list(&mut self) {
        self.lists.open -= 1;
        if self.lists.open == 0 {
            self.lists.root = None;
        }
    }

    pub(super) fn start_item(&mut self) {
        self.lists.items.push(ListItem {
            depth: self.lists.open,
            task_checked: None,
            line: None,
        });
    }

    pub(super) fn end_item(&mut self) {
        self.write_item_line();
        self.lists.items.pop();
    }

    pub(super) fn set_task_checked(&mut self, checked: bool) {
        if let Some(item) = self.lists.items.last_mut() {
            item.task_checked = Some(checked);
        }
    }

    pub(super) fn in_list_item(&self) -> bool {
        !self.lists.items.is_empty()
    }

    /// Write the pending inline content as the innermost item's line, or as
    /// a child of that line when it has already been written.
    pub(super) fn write_item_line(&mut self) {
        let Some(item) = self.lists.items.last() else {
            return;
        };

        if let Some(line) = item.line.clone() {
            if self.pending.is_empty() {
                return;
            }
            let continuation = AstNode::line("", self.line, None, None);
            self.flush_pending_into(&continuation);
            line.add_child(continuation);
            return;
        }

        let depth = item.depth;
        let properties = item.task_checked.map(|checked| self.task_property(checked));
        let line = AstNode::line("", self.line, None, properties);
        self.flush_pending_into(&line);

        let parent = self.lists.root.as_ref().unwrap_or(&self.root);
        add_child_at_depth(parent, line.clone(), depth);
        if let Some(item) = self.lists.items.last_mut() {
            item.line = Some(line);
        }
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
