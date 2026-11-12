import { div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { notify, pickFolder } from "herdr";
import { saveRoots } from "./lib.js";

async function change(view, roots, cx) {
  try {
    await saveRoots(roots);
  } catch (e) {
    notify({ title: "patto: folders not saved", body: String(e.message ?? e), level: "error" });
  }
  view.reload(cx);
}

export function renderFolders(view, t) {
  const roots = view.configured;
  const button = (id, label, onClick) => div().id(id).px(10).py(5).rounded(6).border_1().border_color(t.input)
    .bg(t.raised).cursor_pointer().hover((s) => s.bg(t.hover)).on_click(onClick).child(label);

  return v_flex().p(10).gap(8)
    .child(div().text_size(12).text_color(t.muted)
      .child("Tasks come from these note folders in every space. Links and the preview use the folder of the open note."))
    .children(roots.map((root, i) => h_flex().id(`root-${i}`).gap(8).items_center().px(8).py(5).rounded(6).bg(t.surface)
      .child(div().flex_1().min_w(0).truncate().child(root))
      .child(div().id(`root-rm-${i}`).px(6).rounded(4).cursor_pointer().text_color(t.dim)
        .hover((s) => s.bg(t.hover).text_color(t.foreground))
        .on_click((_e, cx) => cx.spawn((cx) => change(view, roots.filter((r) => r !== root), cx)))
        .child("✕"))))
    .when(!roots.length, (el) => el.child(div().text_size(12).text_color(t.dim)
      .child(view.fallback ? `None set: using this tab's folder, ${view.fallback}.` : "None set.")))
    .child(h_flex().gap(8)
      .child(button("root-add", "Add folder…", (_e, cx) => cx.spawn(async (cx) => {
        const folder = await pickFolder({ prompt: "Add" });
        if (folder && !roots.includes(folder)) await change(view, [...roots, folder], cx);
      })))
      .when(Boolean(view.fallback) && !roots.includes(view.fallback), (el) => el.child(
        button("root-add-tab", "Add this tab's folder", (_e, cx) => cx.spawn((cx) => change(view, [...roots, view.fallback], cx))))))
    .child(div().text_size(11).text_color(t.dim).child("Saved in ~/.config/patto/herdr.json (\"roots\")."));
}
