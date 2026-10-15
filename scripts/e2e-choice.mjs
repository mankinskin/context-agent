import assert from "node:assert/strict";
import { copyFile, mkdir, readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import { fileHash } from "./e2e-runner.mjs";

export async function captureChoice(store, runRoot, live = false, providerEvents = [], partial = false) {
  const sessions = await readdir(join(store, "sessions"));
  assert.equal(sessions.length, 1);
  const target = join(runRoot, "choice");
  await mkdir(target);
  const files = [];
  for (const [name, source] of [
    ["preambles.json", join(store, "preambles.json")],
    ["session.json", join(store, "sessions", sessions[0], "session.json")],
    ["transcript.json", join(store, "sessions", sessions[0], "transcript.json")],
  ]) {
    const path = join(target, name);
    await copyFile(source, path);
    files.push({ path, sha256: await fileHash(path) });
  }
  return { files, observed: partial ? null : await verifyChoice(runRoot, live, providerEvents), partial };
}

export async function verifyChoice(root, live, providerEvents = []) {
  const read = async name => JSON.parse(await readFile(join(root, "choice", name), "utf8"));
  const blocks = await read("preambles.json");
  assert.deepEqual(blocks.map(block => [block.id, block.enabled]), [
    ["PREAMBLE_ENABLED_29AD", true], ["PREAMBLE_DISABLED_29AD", false],
  ]);
  const session = await read("session.json");
  const transcript = await read("transcript.json");
  assert.equal(session.source, "context-agent");
  assert.equal(transcript.session_id, session.session_id);
  const events = transcript.turns.map((turn, index) => {
    const event = JSON.parse(turn.content);
    assert.equal(turn.sequence, index);
    assert.equal(event.sequence, index);
    assert.equal(event.event_id, turn.event_meta.event_id);
    assert.equal(turn.event_meta.parent_event_id ?? null, index ? transcript.turns[index - 1].event_meta.event_id : null);
    return event;
  });
  const assemblies = events.filter(event => event.evidence_kind === "prompt_assembly");
  assert.equal(assemblies.length, live ? 2 : 1);
  for (const assembly of assemblies) {
    assert.deepEqual(assembly.prompt_sources.filter(source => source.source_kind === "preamble").map(source => source.source_id), ["PREAMBLE_ENABLED_29AD"]);
  }
  const content = events.flatMap(event => event.message?.blocks ?? []);
  const widgets = content.filter(block => block.type === "multiple_choice");
  const answers = content.filter(block => block.type === "choice_answer");
  if (!live) {
    assert.deepEqual(events.map(event => event.evidence_kind), ["chat_message", "prompt_assembly", "model_request", "failure"]);
    assert.match(events.at(-1).source_id, /offline_ai_unavailable/);
    assert.equal(widgets.length + answers.length, 0);
  } else {
    assert.equal(widgets.length, 1);
    assert.equal(answers.length, 1);
    assert.equal(answers[0].widget_id, widgets[0].widget_id);
    assert.equal(answers[0].choice, "lookup-a");
    assert.deepEqual(widgets[0].options, ["lookup-a", "lookup-b"]);
    assert.equal(events.filter(event => event.evidence_kind === "model_response").length, 2);
    assert.equal(events.filter(event => event.evidence_kind === "chat_message").length, 2);
    const observations = providerEvents.filter(event => event.kind === "outbound_preamble_observed");
    const sends = providerEvents.filter(event => event.kind === "request_sent");
    assert.equal(observations.length, sends.length);
    assert.ok(sends.length >= 4);
    for (const observation of observations) {
      assert.deepEqual(observation.probes, [
        { id: "PREAMBLE_ENABLED_29AD", present: true },
        { id: "PREAMBLE_DISABLED_29AD", present: false },
      ]);
      assert.match(observation.system_prompt_sha256, /^[a-f0-9]{64}$/);
      assert.ok(sends.some(send => send.request_id === observation.request_id && send.correlation_id === observation.correlation_id && send.sequence > observation.sequence));
    }
  }
  return { session_id: session.session_id, event_ids: events.map(event => event.event_id), widget_id: widgets[0]?.widget_id ?? null, answer: answers[0]?.choice ?? null };
}
