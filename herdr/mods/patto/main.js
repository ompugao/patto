import { View, div } from "gpui-kit";
import { v_flex, h_flex, TextView } from "gpui-base";
import { theme, props, notify, nextCommand, openOverlay, run, watchFolder, layout, shared, setShared } from "herdr";
import { config, current, notesRoot, place, query, renderMarkdown, openInNvim, basename, hasNotes } from "./lib.js";
import { taskRows, renderTasks, nextStatus } from "./tasks.js";
import { linkRows, renderLinks } from "./links.js";
import { Capture } from "./capture.js";

const DONE_RANGES = ["today", "this_week", "none"];
const DONE_LABEL = { today: "done: today", this_week: "done: week", none: "done: off" };

function findBlock(node, match) {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findBlock(child, match);
      if (found) return found;
    }
  } else if (node && typeof node === "object") {
    if (node.type === "block" && match(node)) return node;
    return findBlock(Object.values(node), match);
  }
  return null;
}

export default class Patto extends View {
  init(_props, cx) {
    if (props()?.view === "capture") {
      this.dialog = new Capture();
      this.dialog.init(cx);
      return;
    }
    this.handle = cx.focus_handle();
    this.preview = props()?.view === "preview";
    this.where = this.preview ? props().where : place();
    this.tab = "tasks";
    this.done = "today";
    this.at = 0;
    this.data = null;
    this.links = null;
    this.markdown = "";
    this.file = null;
    this.root = null;
    this.error = null;
    this.watching = new Set();

    this.reload(cx);
    cx.timer.every(2000, (cx) => this.followTab(cx));
    cx.timer.every(60000, (cx) => this.reload(cx));
    cx.spawn(async (cx) => {
      for (;;) {
        const command = await nextCommand();
        this.command(command, cx);
        cx.notify();
      }
    });
  }

  command(command, cx) {
    if (command === "focus") this.handle.focus();
    else if (command === "refresh") this.reload(cx);
    else if (command === "tasks" || command === "links") this.tab = command;
    else if (command === "capture") this.capture();
    else if (command === "preview") this.openPreview();
  }

  followTab(cx) {
    if (this.preview) return this.reloadCurrent(cx);
    const where = place();
    if (where && where.tab !== this.where?.tab) {
      this.where = where;
      this.reload(cx);
    } else {
      this.reloadCurrent(cx);
    }
  }

  // The nvim's file changes far more often than the notes do, so it is
  // checked on its own, and only a change reloads links or the preview.
  reloadCurrent(cx) {
    cx.spawn(async (cx) => {
      const now = await current(this.where);
      if (now?.file && now.file !== this.file && now.file.endsWith(".pn")) {
        this.file = now.file;
        await this.reloadFile();
        cx.notify();
      }
    });
  }

  reload(cx) {
    cx.spawn(async (cx) => {
      try {
        this.cfg = await config();
        const now = await current(this.where);
        if (now?.file?.endsWith(".pn")) this.file = now.file;
        this.root = await notesRoot(this.cfg, this.where, now);
        const notes = Boolean(this.root) && (await hasNotes(this.root));
        if (!this.preview) this.fitPanel(notes);
        if (!notes) {
          this.error = null;
          cx.notify();
          return;
        }
        if (!this.preview) {
          this.data = await query(this.cfg, ["tasks", "--root", this.root, "--done", this.done]);
        }
        await this.reloadFile();
        this.error = null;
        this.watch(this.root, cx);
      } catch (e) {
        this.error = String(e.message ?? e);
      }
      cx.notify();
    });
  }

  // Collapsed to 0 px rather than layout.hide: a hidden block's mod does not
  // start after a restart, so it could never show itself again.
  fitPanel(notes) {
    const block = findBlock(layout(), (b) => b.content?.mod === "patto" && !b.content?.props);
    if (!block) return;
    const px = block.size?.px;
    if (notes && px === 0) {
      run("layout.resize", { id: block.id, px: shared("patto.width") ?? 340 });
    } else if (!notes && px !== 0) {
      if (px) setShared("patto.width", px);
      run("layout.resize", { id: block.id, px: 0 });
    }
  }

  async reloadFile() {
    if (!this.file) return;
    if (this.preview) {
      this.markdown = await renderMarkdown(this.cfg, this.file);
    } else if (this.root) {
      this.links = await query(this.cfg, ["links", "--root", this.root, this.file]);
    }
  }

  watch(dir, cx) {
    if (!dir || this.watching.has(dir)) return;
    this.watching.add(dir);
    cx.spawn(async (cx) => {
      try {
        for (;;) {
          await watchFolder(dir);
          this.reload(cx);
        }
      } catch {
        this.watching.delete(dir);
      }
    });
  }

  openTask(task) {
    openInNvim(this.where, task.file, task.line);
  }

  openTarget(target) {
    openInNvim(this.where, target.file, target.line);
  }

  setStatus(task, status, cx) {
    cx.spawn(async (cx) => {
      try {
        await query(this.cfg, ["set-status", "--file", task.file, "--line", String(task.line), status]);
      } catch (e) {
        notify({ title: "patto", body: String(e.message ?? e), level: "error" });
      }
      this.reload(cx);
    });
  }

  capture() {
    openOverlay({ mod: "patto", kind: "dialog", width: 520, props: { view: "capture", where: this.where } });
  }

  openPreview() {
    run("pane.split", { direction: "right", mod: "patto", props: { view: "preview", where: this.where } });
  }

  rows() {
    return this.tab === "tasks" ? taskRows(this.data) : linkRows(this.links);
  }

  selectable(rows) {
    return rows.filter((r) => (this.tab === "tasks" ? r.kind === "task" : r.target));
  }

  key(e, cx) {
    const items = this.selectable(this.rows());
    const item = items[this.at];
    const k = e.key;
    if (k === "j" || k === "down") this.at = Math.min(this.at + 1, items.length - 1);
    else if (k === "k" || k === "up") this.at = Math.max(this.at - 1, 0);
    else if (k === "enter" && item) this.tab === "tasks" ? this.openTask(item.task) : this.openTarget(item.target);
    else if (k === "x" && item?.task) this.setStatus(item.task, item.task.status === "done" ? "todo" : "done", cx);
    else if (k === "space" && item?.task) this.setStatus(item.task, nextStatus(item.task), cx);
    else if (k === "tab") { this.tab = this.tab === "tasks" ? "links" : "tasks"; this.at = 0; }
    else if (k === "d") { this.done = DONE_RANGES[(DONE_RANGES.indexOf(this.done) + 1) % DONE_RANGES.length]; this.reload(cx); }
    else if (k === "r") this.reload(cx);
    else if (k === "c") this.capture();
    else if (k === "p") this.openPreview();
    else return;
    cx.stop_propagation();
    cx.notify();
  }

  renderPreview(t) {
    return v_flex().id("patto-preview").size_full().bg(t.background).text_color(t.foreground)
      .child(div().px(12).py(6).text_size(12).text_color(t.muted).border_b_1().border_color(t.border)
        .child(this.file ? basename(this.file) : "patto preview"))
      .child(div().id("patto-preview-body").flex_1().min_h(0).p(12).overflow_y_scrollbar()
        .child(this.error ? div().text_color(t.blocked).child(this.error)
          : this.file ? TextView.markdown("patto-md", this.markdown)
          : div().text_color(t.dim).child("Open a note in nvim to preview it.")));
  }

  render() {
    if (this.dialog) return this.dialog.render();
    const t = theme();
    if (this.preview) return this.renderPreview(t);
    const rows = this.rows();
    this.at = Math.max(0, Math.min(this.at, this.selectable(rows).length - 1));
    const tabButton = (id, label) => div().id(`tab-${id}`).px(8).py(3).rounded(5).cursor_pointer()
      .text_color(this.tab === id ? t.foreground : t.dim).when(this.tab === id, (el) => el.bg(t.raised))
      .hover((s) => s.bg(t.hover))
      .on_click((_e, cx) => { this.tab = id; this.at = 0; cx.notify(); })
      .child(label);
    const action = (id, label, onClick) => div().id(id).px(6).py(3).rounded(5).cursor_pointer()
      .text_color(t.muted).hover((s) => s.bg(t.hover).text_color(t.foreground)).on_click(onClick).child(label);

    return v_flex().id("patto").size_full().bg(t.chrome).text_size(13).text_color(t.foreground)
      .track_focus(this.handle).on_key_down((e, cx) => this.key(e, cx))
      .child(h_flex().px(8).py(6).gap(4).items_center().border_b_1().border_color(t.border)
        .child(tabButton("tasks", "Tasks"))
        .child(tabButton("links", "Links"))
        .child(div().flex_1())
        .when(this.tab === "tasks", (el) => el.child(action("done", DONE_LABEL[this.done], (_e, cx) => {
          this.done = DONE_RANGES[(DONE_RANGES.indexOf(this.done) + 1) % DONE_RANGES.length];
          this.reload(cx);
        })))
        .child(action("capture", "+", () => this.capture()))
        .child(action("preview", "Preview", () => this.openPreview())))
      .when(this.tab === "links" && Boolean(this.file), (el) => el.child(
        div().px(10).pt(6).text_size(12).font_semibold().truncate().child(basename(this.file))))
      .child(div().id("patto-list").flex_1().min_h(0).py(4).px(4).overflow_y_scrollbar()
        .child(this.error ? div().p(12).text_color(t.blocked).child(this.error)
          : !this.root ? div().p(12).text_color(t.dim).child("No notes folder for this tab.")
          : this.tab === "tasks" ? renderTasks(this, rows, t)
          : renderLinks(this, rows, t)))
      .child(div().px(10).py(4).text_size(11).text_color(t.dim).border_t_1().border_color(t.border).truncate()
        .child(this.root ?? ""));
  }
}
