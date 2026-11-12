import { appHome, exec, focused, home, readTextFile, writeTextFile, run, spaces } from "herdr";

// Optional settings: {"roots": ["~/notes"], "inbox": "inbox.pn",
// "query_bin": "patto-query", "renderer_bin": "patto-markdown-renderer"}.
const CONFIG_PATH = () => `${home()}/.config/patto/herdr.json`;

export async function config() {
  try {
    return JSON.parse((await readTextFile(CONFIG_PATH())) ?? "{}") ?? {};
  } catch {
    return {};
  }
}

// Note folders shown from every space; empty means "this tab's folder".
export function configuredRoots(cfg) {
  const roots = Array.isArray(cfg.roots) ? cfg.roots : cfg.root ? [cfg.root] : [];
  return roots.filter((r) => typeof r === "string" && r).map(expand);
}

// Rewrites only `roots`, keeping the person's other keys; a concurrent edit
// of the file rejects the write instead of being overwritten.
export async function saveRoots(roots) {
  const path = CONFIG_PATH();
  const before = await readTextFile(path);
  let cfg = {};
  try {
    cfg = JSON.parse(before ?? "{}") ?? {};
  } catch {
    throw new Error(`${path} is not valid JSON; fix it by hand first.`);
  }
  delete cfg.root;
  cfg.roots = roots.map(collapse);
  await exec("mkdir", ["-p", path.slice(0, path.lastIndexOf("/"))]);
  await writeTextFile(path, `${JSON.stringify(cfg, null, 2)}\n`, before);
}

export function rootOf(roots, file) {
  return roots.find((root) => file?.startsWith(`${root}/`)) ?? null;
}

function collapse(path) {
  return path.startsWith(`${home()}/`) ? `~${path.slice(home().length)}` : path;
}

function expand(path) {
  return path?.startsWith("~/") ? `${home()}${path.slice(1)}` : path;
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

// The tab's own notes folder, for when no note folders are configured.
// Remote spaces have none: patto-query reads this machine's files.
export async function tabRoot(where, now) {
  if (spaces().find((s) => s.id === where?.space)?.machine) return null;
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
