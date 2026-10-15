import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { captureNativeSessionTree, verifyNativeSessionTree } from "./e2e-native.mjs";

export async function runSessionNavigation({ run, runBrowser, server, restart, nativeStore, remaining }) {
  let completed = false, browserError;
  const browser = runBrowser("sessions").then(result => {
    completed = true;
    return result;
  }, error => {
    completed = true;
    browserError = error;
  });
  let selection;
  while (!selection) {
    remaining();
    if (completed) throw browserError ?? new Error("browser finished without requesting the owned restart");
    try { selection = JSON.parse(await readFile(join(run.runRoot, "sessions-ready.json"), "utf8")); }
    catch (error) { if (error.code !== "ENOENT") throw error; }
    if (!selection) await new Promise(resolve => setTimeout(resolve, 100));
  }
  const before = await captureNativeSessionTree(nativeStore, run.runRoot, "before", selection);
  const restarted = await restart();
  run.manifest.bridge = { before_pid: server.record.pid, after_pid: restarted.record.pid,
    store_root: nativeStore, selection, before };
  await writeFile(join(run.runRoot, "sessions-restarted.json"), JSON.stringify({ ready: true }));
  const result = await browser;
  if (browserError) throw browserError;
  run.manifest.bridge.after = await captureNativeSessionTree(nativeStore, run.runRoot, "after", selection);
  assert.deepEqual(run.manifest.bridge.after.observed, before.observed, "restart changed native parents or ordered history");
  return result;
}

export async function verifySessionNavigation(runRoot, manifest, cleanup, events) {
  const read = async name => JSON.parse(await readFile(join(runRoot, name), "utf8"));
  const bridge = manifest.bridge;
  assert.ok(bridge.before_pid > 0 && bridge.after_pid > 0 && bridge.before_pid !== bridge.after_pid);
  assert.ok(cleanup.owned_resources.some(entry => entry.pid === bridge.after_pid && entry.closed && entry.absent));
  assert.ok(events.some(entry => entry.kind === "backend_restarted" && entry.before_pid === bridge.before_pid && entry.after_pid === bridge.after_pid));
  const selection = await read("sessions-ready.json");
  assert.deepEqual(selection, bridge.selection);
  assert.ok(selection.url.endsWith(`#session=${selection.selected}`));
  assert.equal((await read("sessions-restarted.json")).ready, true);
  const files = [];
  for (const phase of ["before", "after"]) {
    const record = bridge[phase];
    const records = await Promise.all(record.observed.map(async row => ({
      manifest: await read(join(`native-${phase}`, `${row.session_id}-session.json`)),
      transcript: await read(join(`native-${phase}`, `${row.session_id}-transcript.json`)),
    })));
    assert.deepEqual(verifyNativeSessionTree(records, selection), record.observed);
    files.push(...record.files);
  }
  assert.deepEqual(bridge.after.observed, bridge.before.observed);
  const stores = await read("productive-session-stores.json");
  assert.deepEqual(stores.before, stores.after);
  assert.equal(stores.source_filter, "context-agent");
  assert.equal(stores.before.length, 2);
  return files;
}
