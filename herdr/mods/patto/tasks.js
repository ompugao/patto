import { div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { basename } from "./lib.js";

const GLYPH = { todo: "○", doing: "◐", paused: "‖", done: "✓", archived: "▪" };
const NEXT_STATUS = { todo: "doing", doing: "done", paused: "doing", done: "todo", archived: "todo" };

export function nextStatus(task) {
  return NEXT_STATUS[task.status] ?? "done";
}

// Rows in display order; `task` rows are the selectable ones.
export function taskRows(data) {
  const rows = [];
  const section = (tasks, prefix) => {
    let group = null;
    for (const task of tasks) {
      if (task.group !== group) {
        group = task.group;
        rows.push({ kind: "head", label: prefix + group, group });
      }
      rows.push({ kind: "task", task });
    }
  };
  section(data?.open ?? [], "");
  section(data?.done ?? [], "Done · ");
  return rows;
}

function groupColor(t, group) {
  if (group === "Overdue") return t.blocked;
  if (group === "Today") return t.accent;
  return t.muted;
}

export function renderTasks(view, rows, t) {
  if (!rows.length) {
    return div().p(12).text_color(t.dim).child(view.data ? "No open tasks." : "Loading…");
  }
  let index = -1;
  return v_flex().gap(1).children(rows.map((row, i) => {
    if (row.kind === "head") {
      return div().id(`h-${i}`).pt(i ? 10 : 2).pb(2).px(10).text_size(11).font_semibold()
        .text_color(groupColor(t, row.group)).child(row.label);
    }
    index += 1;
    const at = index;
    const task = row.task;
    const selected = at === view.at;
    const resolved = task.status === "done" || task.status === "archived";
    return h_flex().id(`t-${i}`).px(10).py(3).gap(8).items_center().rounded(5).cursor_pointer()
      .when(selected, (el) => el.bg(t.selected))
      .hover((s) => s.bg(t.hover))
      .on_click((_e, cx) => { view.at = at; view.openTask(task); cx.notify(); })
      .child(div().id(`s-${i}`).w(14).flex_none().text_color(resolved ? t.done : t.foreground)
        .on_click((_e, cx) => { cx.stop_propagation(); view.setStatus(task, nextStatus(task), cx); })
        .child(GLYPH[task.status] ?? "○"))
      .child(v_flex().flex_1().min_w(0)
        .child(div().truncate().text_color(resolved ? t.dim : t.foreground).child(task.text || "(empty)"))
        .child(div().truncate().text_size(11).text_color(t.dim)
          .child([task.due, `${basename(task.file)}:${task.line}`].filter(Boolean).join("  ·  "))));
  }));
}
