import { div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";

// Rows in display order; rows with a `target` are selectable.
export function linkRows(data) {
  if (!data) return [];
  const rows = [{ kind: "head", label: `Back links (${data.backlinks.length})` }];
  for (const link of data.backlinks) {
    rows.push({ kind: "note", indent: 10, label: link.name, target: link.file && { file: link.file, line: link.lines[0]?.line ?? 1 } });
    for (const line of link.lines) {
      rows.push({ kind: "context", indent: 26, label: line.context, target: link.file && { file: link.file, line: line.line } });
    }
  }
  rows.push({ kind: "head", label: `Two-hop links (${data.two_hop.length})` });
  for (const hop of data.two_hop) {
    rows.push({ kind: "bridge", indent: 10, label: `via ${hop.bridge.name}`, target: hop.bridge.file && { file: hop.bridge.file, line: 1 } });
    for (const note of hop.notes) {
      rows.push({ kind: "note", indent: 26, label: note.name, target: note.file && { file: note.file, line: 1 } });
    }
  }
  return rows;
}

export function renderLinks(view, rows, t) {
  if (!view.file) return div().p(12).text_color(t.dim).child("Open a note in nvim to see its links.");
  let index = -1;
  return v_flex().gap(1).children(rows.map((row, i) => {
    if (row.kind === "head") {
      return div().id(`lh-${i}`).pt(i ? 10 : 2).pb(2).px(10).text_size(11).font_semibold().text_color(t.muted).child(row.label);
    }
    if (row.target) index += 1;
    const at = index;
    const selected = row.target && at === view.at;
    return h_flex().id(`l-${i}`).pl(row.indent).pr(10).py(2).rounded(5)
      .when(Boolean(row.target), (el) => el.cursor_pointer().hover((s) => s.bg(t.hover))
        .on_click((_e, cx) => { view.at = at; view.openTarget(row.target); cx.notify(); }))
      .when(selected, (el) => el.bg(t.selected))
      .child(div().truncate()
        .text_size(row.kind === "context" ? 11.5 : 13)
        .text_color(row.kind === "context" ? t.dim : row.kind === "bridge" ? t.muted : row.target ? t.foreground : t.dim)
        .child(row.label || " "));
  }));
}
