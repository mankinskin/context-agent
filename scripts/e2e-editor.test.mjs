import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { test } from "node:test";
import { verifyEditorEvidence } from "./e2e-editor.mjs";

test("editor acceptance rejects labels without actual disk diff sentinel root and link observations", () => {
  const root = join(process.cwd(), "synthetic-editor-fixture");
  const sentinel = "EXTERNAL_SENTINEL_UNCHANGED\n";
  const hash = createHash("sha256").update(sentinel).digest("hex");
  const evidence = {
    git_root: join(root, "git"), git_observed_root: join(root, "git"),
    file: join(root, "git", "notes", "demo.txt"), sentinel: join(root, "editor-sentinel.txt"),
    sentinel_before_sha256: hash, sentinel_after_sha256: hash, foreign_workspace_status: 422,
    rejections: ["../editor-sentinel.txt", "C:/Windows/win.ini", "%2e%2e/editor-sentinel.txt",
      ".git/config", "notes/demo.txt:stream", "notes\\demo.txt", "//server/share/file"]
      .map(path => ({ path, status: 400, code: "editor_path_rejected" })),
    link: { kind: process.platform === "win32" ? "junction" : "symlink", confirmed: true,
      removed: true, statuses: [400, 400], sentinel_unchanged: true },
    valid_file_restored: true,
  };
  const file = "original fixture text\nEDITOR_PROOF_29AD\n";
  const diff = "diff --git a/notes/demo.txt b/notes/demo.txt\n+EDITOR_PROOF_29AD\n";
  verifyEditorEvidence(evidence, file, diff, sentinel, root);
  assert.throws(() => verifyEditorEvidence({}, file, diff, sentinel, root));
  assert.throws(() => verifyEditorEvidence(evidence, "unchanged", diff, sentinel, root));
  assert.throws(() => verifyEditorEvidence(evidence, file, "", sentinel, root));
  assert.throws(() => verifyEditorEvidence(evidence, file, diff, "changed", root));
  for (const mutation of [
    item => { item.git_observed_root = join(root, "foreign"); },
    item => { item.link.confirmed = false; },
    item => { item.link.statuses = [200, 400]; },
    item => { item.link.sentinel_unchanged = false; },
    item => { item.rejections[0].status = 200; },
    item => { item.foreign_workspace_status = 200; },
    item => { item.valid_file_restored = false; },
  ]) {
    const invalid = structuredClone(evidence);
    mutation(invalid);
    assert.throws(() => verifyEditorEvidence(invalid, file, diff, sentinel, root));
  }
});
