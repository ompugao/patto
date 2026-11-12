import { div } from "gpui-kit";
import { v_flex, h_flex, InputState, Input } from "gpui-base";
import { theme, props, notify, closeOverlay } from "herdr";
import { config, configuredRoots, current, tabRoot, place, query, openInNvim, basename } from "./lib.js";

const DUE = /^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2})?$/;

// The quick capture dialog: the patto mod opened as an overlay with props {view: "capture"}.
export class Capture {
  init(cx) {
    this.where = props()?.where ?? place();
    this.task = true;
    this.error = null;
    this.handle = cx.focus_handle();
    this.text = InputState.new({ placeholder: "What needs doing?" });
    this.due = InputState.new({ placeholder: "due YYYY-MM-DD (optional)" });
    for (const input of [this.text, this.due]) input.on("submit", (_e, cx) => this.submit(cx));
    this.text.focus();
    cx.timer.after(300, () => this.text.focus());
    this.roots = [];
    this.target = 0;
    cx.spawn(async (cx) => {
      this.cfg = await config();
      const configured = configuredRoots(this.cfg);
      const fallback = configured.length ? null : await tabRoot(this.where, await current(this.where));
      this.roots = configured.length ? configured : fallback ? [fallback] : [];
      cx.notify();
    });
  }

  close() {
    closeOverlay(props().overlay);
  }

  submit(cx) {
    const text = this.text.value().trim();
    const due = this.due.value().trim();
    if (!text) return;
    if (this.task && due && !DUE.test(due)) {
      this.error = "Use YYYY-MM-DD or YYYY-MM-DDTHH:MM for the due date.";
      cx.notify();
      return;
    }
    cx.spawn(async (cx) => {
      try {
        const cfg = this.cfg ?? (await config());
        const root = this.roots[this.target];
        if (!root) throw new Error("No note folders. Add one with ⚙ in the patto panel.");
        const inbox = (cfg.inbox ?? "inbox.pn").startsWith("/") ? cfg.inbox : `${root}/${cfg.inbox ?? "inbox.pn"}`;
        const args = ["capture", "--file", inbox];
        if (this.task) args.push("--task");
        if (this.task && due) args.push("--due", due);
        const out = await query(cfg, [...args, "--", text]);
        this.close();
        notify({ title: `Added to ${basename(out.file)}`, body: out.text, level: "success", timeout: 4000 });
        if (this.openAfter) openInNvim(this.where, out.file, out.line);
      } catch (e) {
        this.error = String(e.message ?? e);
        cx.notify();
      }
    });
  }

  render() {
    const t = theme();
    const toggle = (id, label, on, flip) => h_flex().id(id).gap(6).items_center().cursor_pointer()
      .text_color(on ? t.foreground : t.dim).on_click((_e, cx) => { flip(); cx.notify(); })
      .child(div().w(14).child(on ? "☑" : "☐")).child(label);
    const field = (state) => Input.new(state).h(36).px(10).rounded(9).border_1()
      .border_color(t.input).bg(t.background).text_size(12.8).text_color(t.foreground);

    return v_flex().id("patto-capture").track_focus(this.handle).w_full().p(18).gap(10)
      .bg(t.popover).text_color(t.foreground).text_size(13)
      .on_key_down((e, cx) => {
        if (e.key === "escape") this.close();
        else if (e.key === "t" && e.modifiers.control) this.task = !this.task;
        else return;
        cx.stop_propagation();
        cx.notify();
      })
      .child(h_flex().items_center()
        .child(div().flex_1().text_size(15).font_semibold().child(this.task ? "New task" : "New note line"))
        .child(div().text_size(11).text_color(t.dim).child("Enter to add · Esc")))
      .when(this.roots.length > 1, (el) => el.child(h_flex().gap(6).items_center().flex_wrap()
        .child(div().text_size(12).text_color(t.dim).child("Into"))
        .children(this.roots.map((root, i) => div().id(`cap-root-${i}`).px(8).py(2).rounded(5).cursor_pointer()
          .border_1().border_color(i === this.target ? t.accent : t.input)
          .text_color(i === this.target ? t.foreground : t.muted)
          .on_click((_e, cx) => { this.target = i; cx.notify(); })
          .child(basename(root))))))
      .child(field(this.text))
      .when(this.task, (el) => el.child(field(this.due)))
      .child(h_flex().gap(16)
        .child(toggle("cap-task", "Task (ctrl+t)", this.task, () => { this.task = !this.task; }))
        .child(toggle("cap-open", "Open in nvim after", Boolean(this.openAfter), () => { this.openAfter = !this.openAfter; })))
      .when(Boolean(this.error), (el) => el.child(div().text_color(t.blocked).child(this.error)));
  }
}
