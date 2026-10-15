import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, readdir } from "node:fs/promises";
import { join } from "node:path";

export async function snapshotSessionStores(roots, excludedSessionId, sourceFilter) {
  const stores = [];
  for (const root of roots) {
    let entries;
    try { entries = await readdir(root, { recursive: true, withFileTypes: true }); }
    catch (error) {
      if (error.code !== "ENOENT") throw error;
      stores.push({ root, exists: false, files: [] });
      continue;
    }
    const selectedDirectories = new Set();
    if (sourceFilter) {
      for (const entry of entries) {
        if (entry.isFile() && entry.name === "session.json"
          && !join(entry.parentPath, entry.name).split(/[\\/]/).includes(excludedSessionId)) {
          const record = JSON.parse(await readFile(join(entry.parentPath, entry.name), "utf8"));
          if (record.source === sourceFilter) selectedDirectories.add(entry.parentPath);
        }
      }
    }
    const files = [];
    for (const entry of entries) {
      const path = join(entry.parentPath, entry.name);
      if (!entry.isFile() || path.split(/[\\/]/).includes(excludedSessionId)) continue;
      if (sourceFilter && !selectedDirectories.has(entry.parentPath)) continue;
      const bytes = await readFile(path);
      files.push({ path, sha256: createHash("sha256").update(bytes).digest("hex") });
    }
    stores.push({ root, exists: true, files: files.sort((a, b) => a.path.localeCompare(b.path)) });
  }
  return stores;
}

export function verifyNativeBridge(manifest, transcript, workspace) {
  assert.equal(manifest.source, "context-agent");
  assert.equal(transcript.session_id, manifest.session_id);
  assert.equal(manifest.metadata.workspace_path, workspace);
  assert.equal(manifest.metadata.agent_id, "context-agent");
  assert.equal(manifest.metadata.producer, "context-agent");
  assert.ok(Date.parse(manifest.started_at));
  assert.equal(transcript.turns.length, 4);
  const events = transcript.turns.map((turn, index) => {
    assert.equal(turn.sequence, index);
    assert.equal(turn.role, index === 0 ? "user" : "system");
    const event = JSON.parse(turn.content);
    assert.equal(event.sequence, index);
    assert.equal(event.session_id, transcript.session_id);
    assert.equal(turn.event_meta.event_id, event.event_id);
    assert.equal(turn.event_meta.message_id, "browser-message-offline native persistence proof");
    assert.equal(turn.event_meta.parent_event_id ?? null, index === 0 ? null : transcript.turns[index - 1].event_meta.event_id);
    return event;
  });
  assert.equal(new Set(events.map((event) => event.event_id)).size, 4);
  assert.deepEqual(events.map((event) => event.evidence_kind), ["chat_message", "prompt_assembly", "model_request", "failure"]);
  assert.equal(events[0].message.blocks[0].text, "offline native persistence proof");
  assert.equal(events[0].message.role, "user");
  assert.ok(events[1].prompt_sources.some((source) => source.source_id === "rust-best-practices"));
  assert.match(events[3].source_id, /offline_ai_unavailable/);
  assert.equal(events.filter((event) => event.message?.role === "assistant").length, 0);
  return { session_id: transcript.session_id, started_at: manifest.started_at, parent_session_id: manifest.parent_session_id ?? null,
    event_ids: events.map((event) => event.event_id), turn_count: transcript.turns.length };
}

export async function captureNativeBridge(store, runRoot, phase) {
  const sessions = (await readdir(join(store, "sessions"), { withFileTypes: true })).filter((entry) => entry.isDirectory());
  assert.equal(sessions.length, 1, "the browser must create exactly one native session");
  const source = join(store, "sessions", sessions[0].name);
  const destination = join(runRoot, `native-${phase}`);
  await mkdir(destination);
  const files = [];
  for (const name of ["session.json", "transcript.json"]) {
    const path = join(destination, name);
    await copyFile(join(source, name), path);
    const bytes = await readFile(path);
    files.push({ path, source_path: join(source, name), sha256: createHash("sha256").update(bytes).digest("hex") });
  }

  const manifest = JSON.parse(await readFile(files[0].path, "utf8"));
  const transcript = JSON.parse(await readFile(files[1].path, "utf8"));
  return { files, observed: verifyNativeBridge(manifest, transcript, store) };
}

export function verifyNativeSessionTree(records, expected) {
  const parents = new Map([[expected.rootA, null], [expected.childA, expected.rootA],
    [expected.grandchildA, expected.childA], [expected.rootB, null]]);
  assert.equal(parents.size, 4, "four distinct UI-created sessions required");
  assert.equal(records.length, 4);
  assert.equal(expected.selected, expected.grandchildA);
  const observed = records.map(({ manifest, transcript }) => {
    assert.ok(parents.has(manifest.session_id));
    assert.equal(manifest.source, "context-agent");
    assert.equal(manifest.parent_session_id ?? null, parents.get(manifest.session_id));
    assert.equal(transcript.session_id, manifest.session_id);
    // Native schema v1 omits `turns` on an empty, newly created session.
    if (transcript.turns === undefined) assert.equal(transcript.schema_version, 1);
    const turns = transcript.turns ?? [];
    const eventIds = turns.map((turn, index) => {
      const event = JSON.parse(turn.content);
      assert.equal(turn.sequence, index);
      assert.equal(event.sequence, index);
      assert.equal(event.session_id, manifest.session_id);
      assert.equal(turn.event_meta.event_id, event.event_id);
      assert.equal(turn.event_meta.parent_event_id ?? null, index ? turns[index - 1].event_meta.event_id : null);
      return event.event_id;
    });
    assert.equal(new Set(eventIds).size, eventIds.length);
    if (manifest.session_id === expected.grandchildA) {
      const events = turns.map(turn => JSON.parse(turn.content));
      assert.deepEqual(events.map(event => event.evidence_kind), ["chat_message", "prompt_assembly", "model_request", "failure"]);
      assert.equal(events[0].message.blocks[0].text, "offline nested branch proof");
      assert.match(events[3].source_id, /offline_ai_unavailable/);
    } else assert.equal(eventIds.length, 0);
    return { session_id: manifest.session_id, parent_session_id: manifest.parent_session_id ?? null,
      event_ids: eventIds, turn_count: turns.length };
  });
  assert.equal(new Set(observed.map(record => record.session_id)).size, 4);
  return observed.sort((left, right) => left.session_id.localeCompare(right.session_id));
}

export async function captureNativeSessionTree(store, runRoot, phase, expected) {
  const sessions = (await readdir(join(store, "sessions"), { withFileTypes: true }))
    .filter((entry) => entry.isDirectory()).map((entry) => entry.name).sort();
  assert.equal(sessions.length, 4, "the UI must create four native sessions");
  const destination = join(runRoot, `native-${phase}`);
  await mkdir(destination);
  const records = [], files = [];
  for (const id of sessions) {
    const source = join(store, "sessions", id);
    const manifest = JSON.parse(await readFile(join(source, "session.json"), "utf8"));
    const transcript = JSON.parse(await readFile(join(source, "transcript.json"), "utf8"));
    records.push({ manifest, transcript });
    for (const name of ["session.json", "transcript.json"]) {
      const path = join(destination, `${id}-${name}`);
      await copyFile(join(source, name), path);
      files.push({ path, sha256: createHash("sha256").update(await readFile(path)).digest("hex") });
    }
  }
  return { files, observed: verifyNativeSessionTree(records, expected) };
}
