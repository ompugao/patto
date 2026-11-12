import { appHome, exec, focused, home, readTextFile, run } from "herdr";

// Optional settings: {"root": "~/notes", "inbox": "inbox.pn",
// "query_bin": "patto-query", "renderer_bin": "patto-markdown-renderer"}.
const CONFIG_PATH = () => `${home()}/.config/patto/herdr.json`;

export async function config() {
  try {
    return JSON.parse((await readTextFile(CONFIG_PATH())) ?? "{}") ?? {};
  } catch {
    return {};
  }
}

function expand(path) {
  return path?.startsWith("~/") ? `${home()}${path.slice(1)}` : path;
}

export function dirname(path) {
  return path.slice(0, Math.max(1, path.lastIndexOf("/")));
}

export function basename(path) {
  return path.slice(path.lastIndexOf("/") + 1).replace(/\.pn$/, "");
}

// The focused tab, captured before any await: the person may switch tabs meanwhile.
export function place(f = focused()) {
  if (!f?.space || !f?.tab) return null;
  const terminal = f.terminal ?? null;
  return { space: f.space, tab: f.tab, terminal, cwd: terminal ? f.cwd ?? null : null };
}

const nvimBin = () => `${appHome()}/mods/herdr-nvim/bin/herdr-nvim`;

// What the tab's herdr-nvim shows: {file, watch, root}, or null without herdr-nvim.
export async function current(where) {
  if (!where) return null;
  try {
    const r = await exec(nvimBin(), ["app-current", "--ctx", JSON.stringify(where)]);
    if (r.code !== 0) return null;
    const out = JSON.parse(r.stdout);
    return { file: out.file ?? null, watch: out.watch ?? null, root: out.tab_root ?? null };
  } catch {
    return null;
  }
}

export async function notesRoot(cfg, where, now) {
  if (cfg.root) return expand(cfg.root);
  if (now?.root) return now.root;
  if (!where?.cwd) return null;
  const r = await exec("git", ["rev-parse", "--show-toplevel"], { cwd: where.cwd }).catch(() => null);
  return r?.code === 0 ? r.stdout.trim() : where.cwd;
}

// The app may start without the login shell's PATH, so cargo's and the
// user's bin folders are tried after the bare name.
const resolved = new Map();
async function runBin(configured, name, args) {
  const candidates = configured ? [expand(configured)] : [name, `${home()}/.cargo/bin/${name}`, `${home()}/.local/bin/${name}`];
  const known = resolved.get(name);
  let lastError = null;
  for (const bin of known ? [known] : candidates) {
    try {
      const r = await exec(bin, args);
      resolved.set(name, bin);
      return r;
    } catch (e) {
      lastError = e;
    }
  }
  throw new Error(`${name} not found (tried ${candidates.join(", ")}): ${lastError}`);
}

export async function query(cfg, args) {
  const r = await runBin(cfg.query_bin, "patto-query", args);
  if (r.code !== 0) throw new Error((r.stderr || r.stdout).trim() || `patto-query exited ${r.code}`);
  return JSON.parse(r.stdout);
}

export async function renderMarkdown(cfg, file) {
  const r = await runBin(cfg.renderer_bin, "patto-markdown-renderer", ["-f", file, "-F", "github"]);
  if (r.code !== 0) throw new Error(r.stderr.trim() || `renderer exited ${r.code}`);
  return r.stdout;
}

// herdr-nvim keeps one nvim per tab; without it, a plain nvim opens beside.
export async function openInNvim(where, file, line = 1) {
  if (where) {
    const r = await exec(nvimBin(), ["open", "--tab", where.tab, "--space", where.space, `${file}:${line}`])
      .catch(() => null);
    if (r?.code === 0) return;
  }
  run("terminal.open", { argv: ["nvim", `+${line}`, file], place: "right" });
}
