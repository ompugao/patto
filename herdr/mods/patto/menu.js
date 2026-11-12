import { div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { theme, props, notify, closeOverlay, shared, setShared } from "herdr";
import { config, query, openInNvim } from "./lib.js";
import { GLYPH } from "./tasks.js";

const STATUSES = ["todo", "doing", "paused", "done", "archived"];

// Bumped after a change so the panel reloads without waiting for the folder watch.
export const CHANGED = "patto.changed";

// The right-click menu of a task: the patto mod opened as a popup with props {view: "menu", task, where}.
export class TaskMenu {
  init(cx) {
    this.task = props().task;
    this.where = props().where;
    this.handle = cx.focus_handle();
    this.handle.focus();
  }

  close() {
    closeOverlay(props().overlay);
  }

  // Closes only after the change: closing the popup cancels its tasks.
  choose(status, cx) {
    if (status === this.task.status) return this.close();
    if (this.busy) return;
    this.busy = status;
    cx.notify();
    cx.spawn(async () => {
      try {
        const args = ["set-status", "--file", this.task.file, "--line", String(this.task.line), status];
        await query(await config(), args, this.task.machine ?? null);
        setShared(CHANGED, (shared(CHANGED) ?? 0) + 1);
      } catch (e) {
        notify({ title: "patto: status not changed", body: String(e.message ?? e), level: "error" });
      }
      this.close();
    });
  }

  render() {
    const t = theme();
    const item = (id, glyph, label, current, onClick) => h_flex().id(id).gap(8).px(10).py(5).rounded(5).items_center()
      .cursor_pointer().hover((s) => s.bg(t.hover)).on_click(onClick)
      .child(div().w(14).text_color(current ? t.accent : t.muted).child(glyph))
      .child(div().flex_1().text_color(current ? t.foreground : t.muted).child(label))
      .when(current, (el) => el.child(div().text_size(11).text_color(t.dim).child("now")))
      .when(this.busy === label, (el) => el.child(div().text_size(11).text_color(t.dim).child("…")));

    return v_flex().id("patto-menu").track_focus(this.handle).w_full().p(4).gap(1).bg(t.popover)
      .text_size(13).text_color(t.foreground)
      .on_key_down((e, cx) => {
        if (e.key !== "escape") return;
        this.close();
        cx.stop_propagation();
      })
      .children(STATUSES.map((status) => item(`menu-${status}`, GLYPH[status], status,
        status === this.task.status, (_e, cx) => this.choose(status, cx))))
      .child(div().h(1).my(3).bg(t.border))
      .child(item("menu-open", "↗", "Open in nvim", false, () => {
        this.close();
        openInNvim(this.where, this.task.file, this.task.line);
      }));
  }
}
