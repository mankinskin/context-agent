import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFile, lstat, mkdir, readFile, realpath, rename, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const sentinelText = "EXTERNAL_SENTINEL_UNCHANGED\n";
const rejectedPaths = ["../editor-sentinel.txt", "C:/Windows/win.ini", "%2e%2e/editor-sentinel.txt",
  ".git/config", "notes/demo.txt:stream", "notes\\demo.txt", "//server/share/file"];

export async function provisionEditor(run, root) {
  await mkdir(join(root, "notes"));
  await writeFile(join(root, "notes", "demo.txt"), "original fixture text\n");
  await writeFile(join(run.fixtureRoot, "editor-sentinel.txt"), sentinelText);
  await run.spawn("git", ["init"], { cwd: root, timeoutMs: 30_000 });
  await run.spawn("git", ["add", "notes/demo.txt"], { cwd: root, timeoutMs: 30_000 });
  await run.spawn("git", ["-c", "user.name=Context Agent", "-c", "user.email=context-agent@example.invalid", "commit", "-m", "editor baseline"], { cwd: root, timeoutMs: 30_000 });
}

export function verifyEditorEvidence(evidence, file, diff, sentinel, fixtureRoot) {
  const root = resolve(fixtureRoot, "git");
  assert.equal(evidence.git_root, root);
  assert.equal(evidence.git_observed_root, root);
  assert.equal(evidence.file, join(root, "notes", "demo.txt"));
  assert.equal(evidence.sentinel, join(fixtureRoot, "editor-sentinel.txt"));
  assert.match(file, /EDITOR_PROOF_29AD/);
  assert.match(diff, /\+EDITOR_PROOF_29AD/);
  assert.match(diff, /diff --git a\/notes\/demo.txt b\/notes\/demo.txt/);
  assert.equal(sentinel, sentinelText);
  assert.equal(evidence.sentinel_before_sha256, hash(Buffer.from(sentinelText)));
  assert.equal(evidence.sentinel_after_sha256, evidence.sentinel_before_sha256);
  assert.deepEqual(evidence.rejections.map(item => item.path), rejectedPaths);
  for (const item of evidence.rejections) {
    assert.ok(item.status >= 400 && item.status < 500);
    assert.equal(item.code, "editor_path_rejected");
  }
  assert.ok(evidence.foreign_workspace_status >= 400 && evidence.foreign_workspace_status < 500);
  assert.equal(evidence.link.kind, process.platform === "win32" ? "junction" : "symlink");
  assert.equal(evidence.link.confirmed, true);
  assert.equal(evidence.link.removed, true);
  assert.deepEqual(evidence.link.statuses, [400, 400]);
  assert.equal(evidence.link.sentinel_unchanged, true);
  assert.equal(evidence.valid_file_restored, true);
}

export async function captureEditor(run, base, remaining) {
  const root = run.manifest.fixture_proof.git_root;
  const file = join(root, "notes", "demo.txt");
  const sentinel = run.manifest.fixture_proof.sentinel;
  const target = join(run.runRoot, "editor");
  await mkdir(target);
  const content = await readFile(file, "utf8");
  const before = await readFile(sentinel);
  assert.equal(before.toString(), sentinelText);
  const git = async args => (await run.spawn("git", ["--no-pager", "-C", root, "-c", "core.fsmonitor=false", ...args],
    { role: "editor-maintainer", timeoutMs: Math.min(5000, remaining()), env: { GIT_EXTERNAL_DIFF: "", GIT_DIFF_OPTS: "" } })).stdout;
  const observedRoot = await realpath((await git(["rev-parse", "--show-toplevel"])).trim());
  const diff = await git(["diff", "--no-ext-diff", "--no-textconv", "--no-color", "--", "notes/demo.txt"]);
  const request = async (method, path, extra = {}) => {
    const response = await fetch(`${base}/api/editor/${method === "GET" ? `file?path=${encodeURIComponent(path)}` : "file"}`, {
      method, headers: { "content-type": "application/json" },
      ...(method === "PUT" ? { body: JSON.stringify({ path, content: "FORBIDDEN", ...extra }) } : {}),
      signal: AbortSignal.timeout(Math.min(3000, remaining())),
    });
    return { path, status: response.status, code: (await response.json()).code };
  };
  const rejections = [];
  for (const path of rejectedPaths) rejections.push(await request("GET", path));
  const foreign = await request("PUT", "notes/demo.txt", { workspace: "foreign" });
  const notes = join(root, "notes");
  const backup = join(root, "owned-notes");
  const outside = join(run.fixtureRoot, "outside");
  await mkdir(outside);
  await copyFile(sentinel, join(outside, "demo.txt"));
  await rename(notes, backup);
  const link = { kind: process.platform === "win32" ? "junction" : "symlink", statuses: [] };
  try {
    if (process.platform === "win32") {
      await run.spawn("cmd.exe", ["/c", "mklink", "/J", notes, outside],
        { role: "editor-junction", timeoutMs: Math.min(5000, remaining()) });
    } else {
      const { symlink } = await import("node:fs/promises");
      await symlink(outside, notes);
    }
    link.confirmed = (await lstat(notes)).isSymbolicLink();
    link.statuses = [(await request("GET", "notes/demo.txt")).status, (await request("PUT", "notes/demo.txt")).status];
    link.sentinel_unchanged = (await readFile(join(outside, "demo.txt"), "utf8")) === sentinelText;
  } finally {
    // Remove only the exact fixture link, never recursively follow the junction.
    try {
      if ((await lstat(notes)).isSymbolicLink()) await rm(notes);
    } catch (error) { if (error.code !== "ENOENT") throw error; }
    await rename(backup, notes);
  }
  link.removed = !(await lstat(notes)).isSymbolicLink();
  const after = await readFile(sentinel);
  const evidence = { git_root: resolve(root), git_observed_root: resolve(observedRoot), file, sentinel,
    sentinel_before_sha256: hash(before), sentinel_after_sha256: hash(after), rejections,
    foreign_workspace_status: foreign.status, link, valid_file_restored: await readFile(file, "utf8") === content };
  verifyEditorEvidence(evidence, content, diff, after.toString(), run.fixtureRoot);
  const files = [];
  for (const [name, bytes] of [["demo.txt", content], ["diff.patch", diff], ["sentinel.txt", after],
    ["readback.json", JSON.stringify(evidence, null, 2)]]) {
    const path = join(target, name);
    await writeFile(path, bytes);
    files.push({ path, sha256: hash(Buffer.from(bytes)) });
  }
  return { files, observed: evidence };
}

export async function verifyEditor(root, manifest) {
  const read = name => readFile(join(root, "editor", name), "utf8");
  const evidence = JSON.parse(await read("readback.json"));
  verifyEditorEvidence(evidence, await read("demo.txt"), await read("diff.patch"), await read("sentinel.txt"), manifest.fixture_root);
  assert.deepEqual(evidence, manifest.editor.observed);
  assert.deepEqual(manifest.editor.files.map(item => item.path),
    ["demo.txt", "diff.patch", "sentinel.txt", "readback.json"].map(name => join(root, "editor", name)));
  return manifest.editor.files;
}
