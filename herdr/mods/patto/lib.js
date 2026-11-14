import { appHome, exec, focused, home, notify, readTextFile, writeTextFile, run, spaces } from "herdr";

// Optional settings: {"roots": ["~/notes", {"path": "~/notes", "machine": "work"}], "inbox": "inbox.pn",
// "query_bin": "patto-query", "renderer_bin": "patto-markdown-renderer"}.
const CONFIG_PATH = () => `${home()}/.config/patto/herdr.json`;

export async function config() {
  try {
    return JSON.parse((await readTextFile(CONFIG_PATH())) ?? "{}") ?? {};
  } catch {
    return {};
  }
}

// Note folders shown from every space, as {path, machine} (machine null:
// this computer); empty means "this tab's folder". A top-level "machine" is
// where plain-string folders live. Paths keep their "~": it is expanded on
// the machine that holds them.
export function configuredRoots(cfg) {
  const roots = Array.isArray(cfg.roots) ? cfg.roots : cfg.root ? [cfg.root] : [];
  const machine = typeof cfg.machine === "string" && cfg.machine ? cfg.machine : null;
  return roots
    .map((r) => (typeof r === "string" ? { path: r, machine } : { path: r?.path, machine: r?.machine ?? null }))
    .filter((r) => typeof r.path === "string" && r.path);
}

export function sameRoot(a, b) {
  return a?.path === b?.path && (a?.machine ?? null) === (b?.machine ?? null);
}

export function rootLabel(root) {
  return root.machine ? `${root.machine}:${root.path}` : root.path;
}

export function shortRootLabel(root) {
  return root.machine ? `${basename(root.path)} (${root.machine})` : basename(root.path);
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
  const plain = typeof cfg.machine === "string" && cfg.machine ? cfg.machine : null;
  cfg.roots = roots.map((r) => ((r.machine ?? null) === plain ? r.path : { path: r.path, machine: r.machine ?? null }));
  await exec("mkdir", ["-p", path.slice(0, path.lastIndexOf("/"))]);
  await writeTextFile(path, `${JSON.stringify(cfg, null, 2)}\n`, before);
}

// The listed folder holding `file` on `machine`; roots must be resolved.
export function rootOf(roots, file, machine) {
  return roots.find((root) => (root.machine ?? null) === (machine ?? null) && file?.startsWith(`${root.full}/`)) ?? null;
}

const homes = new Map();
async function homeOn(machine) {
  if (!machine) return home();
  if (!homes.has(machine)) {
    const r = await exec("sh", ["-c", 'printf %s "$HOME"'], { on: machine });
    if (r.code !== 0 || !r.stdout) throw new Error(`cannot read $HOME on ${machine}`);
    homes.set(machine, r.stdout);
  }
  return homes.get(machine);
}

async function expand(path, machine) {
  return path?.startsWith("~/") ? `${await homeOn(machine)}${path.slice(1)}` : path;
}

// The root with `full`, its path as that machine names it.
export async function resolveRoot(root) {
  return { ...root, full: await expand(root.path, root.machine) };
}

export function machineOf(where) {
  return spaces().find((s) => s.id === where?.space)?.machine ?? null;
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

// The tab's own notes folder, on the tab's machine, for when no note folders are configured.
export async function tabRoot(where, now) {
  const machine = machineOf(where);
  if (now?.root) return { path: now.root, machine };
  if (!where?.cwd) return null;
  const r = await exec("git", ["rev-parse", "--show-toplevel"], { cwd: where.cwd, on: machine }).catch(() => null);
  return { path: r?.code === 0 ? r.stdout.trim() : where.cwd, machine };
}

// The app may start without the login shell's PATH, so cargo's and the
// user's bin folders are tried after the bare name.
const resolved = new Map();
async function runBin(configured, name, args, machine) {
  const where = machine ? ` on ${machine}` : "";
  const dir = await homeOn(machine);
  const candidates = configured ? [await expand(configured, machine)] : [name, `${dir}/.cargo/bin/${name}`, `${dir}/.local/bin/${name}`];
  const key = `${machine ?? ""}/${name}`;
  const known = resolved.get(key);
  let lastError = null;
  for (const bin of known ? [known] : candidates) {
    try {
      const r = await exec(bin, args, { on: machine });
      // A remote shell reports a missing program as exit 127 instead of rejecting.
      if (r.code === 127) throw new Error(r.stderr.trim());
      resolved.set(key, bin);
      return r;
    } catch (e) {
      lastError = e;
    }
  }
  throw new Error(`${name} not found${where} (tried ${candidates.join(", ")}): ${lastError?.message ?? lastError}`);
}

// Runs patto-query on `machine` (null: this computer).
export async function query(cfg, args, machine = null) {
  const r = await runBin(cfg.query_bin, "patto-query", args, machine);
  if (r.code !== 0) throw new Error((r.stderr || r.stdout).trim() || `patto-query exited ${r.code}`);
  return JSON.parse(r.stdout);
}

export async function renderMarkdown(cfg, file, machine = null) {
  const r = await runBin(cfg.renderer_bin, "patto-markdown-renderer", ["-f", file, "-F", "github"], machine);
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

// Finds an existing space on targetMachine (null: this computer).
// If preferFolder is given, prefers a space whose folder matches or is within preferFolder.
export function findSpaceForMachine(targetMachine, preferFolder = null) {
  const want = targetMachine ?? null;
  const list = spaces().filter((s) => (s.machine ?? null) === want);
  if (!list.length) return null;
  if (preferFolder) {
    const match = list.find((s) => s.folder && (s.folder === preferFolder || preferFolder.startsWith(s.folder)));
    if (match) return match;
  }
  return list.find((s) => s.active) ?? list[0];
}

// Opens file:line in nvim on targetMachine (null: this computer), routing to an
// appropriate space on that machine or creating one.
export async function openInMachineNvim(targetMachine, file, line = 1, preferFolder = null) {
  const wantMachine = targetMachine ?? null;
  const currentSpace = spaces().find((s) => s.active);
  const curMachine = currentSpace?.machine ?? null;

  if (curMachine === wantMachine && currentSpace) {
    const f = place();
    if (wantMachine === null) {
      return openInNvim(f, file, line);
    }
    // Remote machine in the current space: open a tab with nvim
    run("terminal.open", { argv: ["nvim", `+${line}`, file], place: "right" });
    return;
  }

  const targetSpace = findSpaceForMachine(wantMachine, preferFolder);
  if (targetSpace) {
    run("space.select", { id: targetSpace.id });
    if (wantMachine === null) {
      const tab = targetSpace.tabs?.find((t) => t.active) ?? targetSpace.tabs?.[0];
      const where = tab ? { space: targetSpace.id, tab: tab.id, terminal: null, cwd: null } : null;
      await openInNvim(where, file, line);
    } else {
      run("tab.new", { space: targetSpace.id, argv: ["nvim", `+${line}`, file] });
    }
    notify({
      title: "patto",
      body: `Opened on ${wantMachine ?? "this computer"} (${targetSpace.name})`,
      timeout: 3000,
    });
  } else {
    run("space.new", {
      machine: wantMachine ?? "",
      folder: preferFolder ?? "",
      argv: ["nvim", `+${line}`, file],
    });
    notify({
      title: "patto",
      body: `Created space on ${wantMachine ?? "this computer"}`,
      timeout: 3000,
    });
  }
}
