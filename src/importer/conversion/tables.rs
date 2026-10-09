use super::Conversion;
use crate::parser::AstNode;

pub(super) struct Table {
    pub(super) node: AstNode,
    pub(super) row: Option<AstNode>,
    pub(super) cell: Option<AstNode>,
}

impl Conversion<'_> {
    pub(super) fn start_table(&mut self) {
        self.table = Some(Table {
            node: AstNode::table("", self.line, None, None),
            row: None,
            cell: None,
        });
        self.report.statistics.increment_feature("tables");
    }

    pub(super) fn end_table(&mut self) {
        if let Some(table) = self.table.take() {
            self.add_block_line(table.node);
        }
    }

    pub(super) fn start_table_row(&mut self) {
        let row = AstNode::tablerow("", self.line, None);
        if let Some(table) = self.table.as_mut() {
            table.row = Some(row);
        }
    }

    pub(super) fn end_table_row(&mut self) {
        if let Some(table) = self.table.as_mut() {
            if let Some(row) = table.row.take() {
                table.node.add_child(row);
            }
        }
    }

    pub(super) fn start_table_cell(&mut self) {
        let cell = AstNode::tablecolumn("", self.line, None);
        if let Some(table) = self.table.as_mut() {
            table.cell = Some(cell);
        }
    }

    pub(super) fn end_table_cell(&mut self) {
        if let Some(table) = self.table.as_mut() {
            if let (Some(cell), Some(row)) = (table.cell.take(), table.row.as_ref()) {
                row.add_content(cell);
            }
        }
    }
}
