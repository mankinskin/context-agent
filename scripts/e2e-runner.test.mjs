import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const runnerPath = fileURLToPath(new URL("./e2e-runner.mjs", import.meta.url));
const runner = () => import("./e2e-runner.mjs");

test("native bridge rejects baseline-only evidence without durable failure turns", async () => {
  const { verifyNativeBridge } = await import("./e2e-native.mjs");
  const manifest = { source: "context-agent", session_id: "synthetic-session", started_at: "2026-10-14T00:00:00Z",
    metadata: { workspace_path: "synthetic-store", agent_id: "context-agent", producer: "context-agent" } };
  assert.throws(() => verifyNativeBridge(manifest, { session_id: manifest.session_id, turns: [] }, "synthetic-store"),
    { code: "ERR_ASSERTION" });
});

test("session tree evidence requires exact UI parents and genuine ordered branch events", async () => {
  const { verifyNativeSessionTree } = await import("./e2e-native.mjs");
  const expected = { rootA: "a", childA: "c", grandchildA: "g", rootB: "b", selected: "g" };
  const records = [["a", null], ["c", "a"], ["g", "c"], ["b", null]].map(([id, parent]) => ({
    manifest: { session_id: id, parent_session_id: parent, source: "context-agent" },
    transcript: { session_id: id, turns: id === "g" ? ["chat_message", "prompt_assembly", "model_request", "failure"].map((kind, index) => {
      const event = { event_id: `event-${index}`, session_id: id, sequence: index, evidence_kind: kind,
        source_id: index === 3 ? "offline_ai_unavailable" : "fixture",
        message: index === 0 ? { blocks: [{ text: "offline nested branch proof" }] } : null };
      return { sequence: index, content: JSON.stringify(event),
        event_meta: { event_id: event.event_id, parent_event_id: index ? `event-${index - 1}` : null } };
    }) : [] },
  }));
  assert.equal(verifyNativeSessionTree(records, expected).find(row => row.session_id === "g").turn_count, 4);
  const omittedEmptyTurns = structuredClone(records);
  omittedEmptyTurns[0].transcript.schema_version = 1;
  delete omittedEmptyTurns[0].transcript.turns;
  assert.equal(verifyNativeSessionTree(omittedEmptyTurns, expected).find(row => row.session_id === "a").turn_count, 0);
  const wrongParent = structuredClone(records);
  wrongParent[1].manifest.parent_session_id = "b";
  assert.throws(() => verifyNativeSessionTree(wrongParent, expected));
  const missingEvents = structuredClone(records);
  missingEvents[2].transcript.turns = [];
  assert.throws(() => verifyNativeSessionTree(missingEvents, expected));
  const wrongOrder = structuredClone(records);
  wrongOrder[2].transcript.turns.reverse();
  assert.throws(() => verifyNativeSessionTree(wrongOrder, expected));
});

test("productive store fingerprints detect changed files and document absent roots", async (t) => {
  const root = await fixture(t);
  const { snapshotSessionStores } = await import("./e2e-native.mjs");
  await mkdir(join(root, "controller"));
  await writeFile(join(root, "controller", "capture.json"), "capture-one");
  await writeFile(join(root, "productive.json"), "one");
  const roots = [root, join(root, "absent")];
  const before = await snapshotSessionStores(roots, "controller");
  await writeFile(join(root, "controller", "capture.json"), "capture-two");
  assert.deepEqual(await snapshotSessionStores(roots, "controller"), before);
  await writeFile(join(root, "productive.json"), "two");
  assert.notDeepEqual(await snapshotSessionStores(roots, "controller"), before);
  assert.equal(before[1].exists, false);
});

test("application fingerprints ignore concurrent capture but detect application writes", async (t) => {
  const root = await fixture(t);
  const { snapshotSessionStores } = await import("./e2e-native.mjs");
  for (const [id, source] of [["application", "context-agent"], ["capture", "copilot"]]) {
    await mkdir(join(root, id));
    await writeFile(join(root, id, "session.json"), JSON.stringify({ source }));
    await writeFile(join(root, id, "transcript.json"), "before");
  }
  const before = await snapshotSessionStores([root], "controller", "context-agent");
  assert.equal(before[0].files.length, 2);
  await writeFile(join(root, "capture", "transcript.json"), "after");
  assert.deepEqual(await snapshotSessionStores([root], "controller", "context-agent"), before);
  await writeFile(join(root, "application", "transcript.json"), "after");
  assert.notDeepEqual(await snapshotSessionStores([root], "controller", "context-agent"), before);
});

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), "context-agent-runner-unit-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}

async function failingHealthServer(t) {
  const server = createServer((_request, response) => {
    response.writeHead(503);
    response.end("not ready");
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  t.after(() => new Promise((resolve) => server.close(resolve)));
  return { server, port: server.address().port };
}

test("help exposes the owned-runner interface without starting services", () => {
  const result = spawnSync(process.execPath, [runnerPath, "--help"], {
    encoding: "utf8",
    timeout: 3000,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0);
  assert.match(result.stdout, /--lane/);
  assert.match(result.stdout, /--scenario/);
  assert.match(result.stdout, /sessions/);
  assert.match(result.stdout, /choice-mcp/);
  assert.match(result.stdout, /--artifact-root/);
  assert.match(result.stdout, /--budget-ledger/);
  assert.match(result.stdout, /--media all\|desktop\|mobile\|tablet/);
  assert.match(result.stdout, /--verify-artifacts/);
});

test("offline choice evidence requires actual enabled config and failure without fabricated widgets", async t => {
  const root = await fixture(t);
  const { verifyChoice } = await import("./e2e-choice.mjs");
  await mkdir(join(root, "choice"));
  await writeFile(join(root, "choice", "preambles.json"), JSON.stringify([
    { id: "PREAMBLE_ENABLED_29AD", enabled: true }, { id: "PREAMBLE_DISABLED_29AD", enabled: false },
  ]));
  await writeFile(join(root, "choice", "session.json"), JSON.stringify({ source: "context-agent", session_id: "unit-fixture" }));
  const turns = ["chat_message", "prompt_assembly", "model_request", "failure"].map((kind, index) => ({
    sequence: index, event_meta: { event_id: `event-${index}`, parent_event_id: index ? `event-${index - 1}` : null },
    content: JSON.stringify({ event_id: `event-${index}`, sequence: index, evidence_kind: kind,
      source_id: index === 3 ? "offline_ai_unavailable" : null,
      prompt_sources: kind === "prompt_assembly" ? [{ source_kind: "preamble", source_id: "PREAMBLE_ENABLED_29AD" }] : [] }),
  }));
  const path = join(root, "choice", "transcript.json");
  await writeFile(path, JSON.stringify({ session_id: "unit-fixture", turns }));
  assert.equal((await verifyChoice(root, false)).widget_id, null);
  const event = JSON.parse(turns[3].content);
  event.message = { blocks: [{ type: "multiple_choice", widget_id: "invented" }] };
  turns[3].content = JSON.stringify(event);
  await writeFile(path, JSON.stringify({ session_id: "unit-fixture", turns }));
  await assert.rejects(verifyChoice(root, false));
});

for (const args of [
  ["--lane", "invalid"],
  ["--lane", "offline", "--scenario", "not-implemented"],
  ["--unexpected-flag"],
  ["--lane", "live", "--scenario", "spine", "--media", "all", "--artifact-root", "synthetic", "--budget-ledger", "synthetic"],
  ["--lane", "offline", "--scenario", "baseline", "--media", "watch", "--artifact-root", "synthetic", "--budget-ledger", "synthetic"],
]) {
  test(`invalid arguments fail before builds: ${args.join(" ")}`, () => {
    const result = spawnSync(process.execPath, [runnerPath, ...args], {
      encoding: "utf8",
      timeout: 3000,
    });
    assert.ifError(result.error);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /invalid|unknown|unsupported|unrecognized/i);
  });
}

test("media projects declare the required viewport, input, and orientation matrix", async () => {
  const { mediaProjects } = await runner();
  assert.deepEqual(mediaProjects("desktop"), [{
    name: "desktop", viewport: { width: 1440, height: 900 }, isMobile: false, hasTouch: false, input: "mouse-keyboard", orientation: "landscape",
  }]);
  assert.deepEqual(mediaProjects("mobile").map(({ name, viewport, hasTouch, isMobile }) => ({ name, viewport, hasTouch, isMobile })), [
    { name: "mobile-portrait", viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
    { name: "mobile-landscape", viewport: { width: 844, height: 390 }, hasTouch: true, isMobile: true },
  ]);
  assert.deepEqual(mediaProjects("tablet").map(({ name, viewport, hasTouch, isMobile }) => ({ name, viewport, hasTouch, isMobile })), [
    { name: "tablet-portrait", viewport: { width: 768, height: 1024 }, hasTouch: true, isMobile: true },
    { name: "tablet-landscape", viewport: { width: 1024, height: 768 }, hasTouch: true, isMobile: true },
  ]);
  assert.equal(mediaProjects("all").length, 5);
});

test("an occupied loopback port fails without stopping its owner", async (t) => {
  const { port } = await failingHealthServer(t);
  const { ensurePortAvailable } = await runner();
  await assert.rejects(ensurePortAvailable(port), { code: "PORT_IN_USE" });
  const response = await fetch(`http://127.0.0.1:${port}/health`);
  assert.equal(response.status, 503);
});

test("readiness has a terminal deadline instead of waiting indefinitely", async (t) => {
  const { port } = await failingHealthServer(t);
  const { waitForReady } = await runner();
  const start = performance.now();
  await assert.rejects(
    waitForReady(`http://127.0.0.1:${port}`, { timeoutMs: 150 }),
    { code: "READINESS_TIMEOUT" },
  );
  assert.ok(performance.now() - start < 2000, "readiness must stop promptly");
});

test("timeout cancels and joins the owned process while retaining evidence", async (t) => {
  const root = await fixture(t);
  const ledger = join(root, "live-budget.json");
  const retainedLedger = '{"reserved":7,"sent":3}\n';
  await writeFile(ledger, retainedLedger);
  const { createOwnedRun } = await runner();
  const run = await createOwnedRun({
    artifactRoot: root,
    budgetLedger: ledger,
    runId: randomUUID(),
    lane: "offline",
    scenario: "baseline",
  });
  const pidFile = join(run.fixtureRoot, "owned.pid");
  let cleanup;
  try {
    await writeFile(join(run.runRoot, "retained.txt"), "failure evidence");
    await assert.rejects(
      run.spawn(
        process.execPath,
        ["-e", "require('fs').writeFileSync(process.argv[1], String(process.pid)); setInterval(() => {}, 1000)", pidFile],
        { timeoutMs: 1000 },
      ),
      { code: "PROCESS_TIMEOUT" },
    );
    const pid = Number(await readFile(pidFile, "utf8"));
    assert.ok(Number.isSafeInteger(pid) && pid > 0);
    assert.throws(() => process.kill(pid, 0), { code: "ESRCH" });
  } finally {
    cleanup = await run.cleanup();
  }
  assert.equal(cleanup.remaining_resource_count, 0);
  assert.equal(await readFile(ledger, "utf8"), retainedLedger);
  assert.equal(await readFile(join(run.runRoot, "retained.txt"), "utf8"), "failure evidence");
  await assert.rejects(stat(run.fixtureRoot), { code: "ENOENT" });
  const persistedCleanup = JSON.parse(await readFile(join(run.runRoot, "cleanup.json"), "utf8"));
  assert.equal(persistedCleanup.remaining_resource_count, 0);
});

test("browser and tool children do not inherit provider credentials", async (t) => {
  const root = await fixture(t);
  const ledger = join(root, "live-budget.json");
  await writeFile(ledger, '{"reserved":0,"sent":0}\n');
  const { createOwnedRun } = await runner();
  const run = await createOwnedRun({
    artifactRoot: root,
    budgetLedger: ledger,
    runId: randomUUID(),
    lane: "offline",
    scenario: "baseline",
  });

  try {
    const result = await run.spawn(
      process.execPath,
      ["-e", "console.log(['GITHUB_COPILOT_TOKEN','COPILOT_API_KEY','GITHUB_TOKEN','GH_TOKEN'].some(key => key in process.env))"],
      {
        timeoutMs: 3000,
        env: {
          GITHUB_COPILOT_TOKEN: "synthetic-credential-sentinel",
          COPILOT_API_KEY: "synthetic-credential-sentinel",
          GITHUB_TOKEN: "synthetic-credential-sentinel",
          GH_TOKEN: "synthetic-credential-sentinel",
        },
      },
    );
    assert.equal(result.stdout.trim(), "false");
  } finally {
    await run.cleanup();
  }
});

test("artifact verification rejects an empty evidence root", async (t) => {
    const root = await fixture(t);
    const result = spawnSync(process.execPath, [runnerPath, "--verify-artifacts", root], {
      encoding: "utf8",
      timeout: 3000,
    });
    assert.ifError(result.error);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /artifact|evidence|empty|missing/i);
  });

  test("artifact verification rejects a failed run rather than checking file presence", async (t) => {
    const root = await fixture(t);
    const ledger = join(root, "live-budget.json");
    await writeFile(ledger, '{"reserved":0,"sent":0}\n');
    const { createOwnedRun } = await runner();
    const run = await createOwnedRun({
      artifactRoot: root,
      budgetLedger: ledger,
      runId: randomUUID(),
      lane: "offline",
      scenario: "baseline",
    });
    run.manifest.outcome = "failed";
    await run.persistManifest();
    await writeFile(join(run.runRoot, "assertions.json"), '[{"passed":false}]\n');
    await run.event({ kind: "failed", sequence: 1 });
    await run.cleanup();
    const result = spawnSync(process.execPath, [runnerPath, "--verify-artifacts", run.runRoot], {
      encoding: "utf8",
      timeout: 3000,
    });
    assert.ifError(result.error);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /artifact|evidence|failed|missing/i);
  });

  test("success labels cannot substitute for build identity and browser artifacts", async (t) => {
    const root = await fixture(t);
    const ledger = join(root, "live-budget.json");
    await writeFile(ledger, '{"reserved":0,"sent":0}\n');
    const { createOwnedRun } = await runner();
    const run = await createOwnedRun({
      artifactRoot: root,
      budgetLedger: ledger,
      runId: randomUUID(),
      lane: "offline",
      scenario: "baseline",
    });
    run.manifest.outcome = "passed";
    await run.persistManifest();
    await writeFile(join(run.runRoot, "assertions.json"), '[{"passed":true}]\n');
    await run.event({ kind: "passed", sequence: 1 });
    await mkdir(join(run.runRoot, "browser"), { recursive: true });
    await run.cleanup();
    const result = spawnSync(process.execPath, [runnerPath, "--verify-artifacts", run.runRoot], {
      encoding: "utf8",
      timeout: 3000,
    });
    assert.ifError(result.error);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /artifact|evidence|identity|missing/i);
  });

test("cleanup observes service descendants and preserves exited process history", async (t) => {
  const root = await fixture(t);
  const ledger = join(root, "live-budget.json");
  await writeFile(ledger, '{"reserved":7,"sent":3}\n');
  const { createOwnedRun } = await runner();
  const run = await createOwnedRun({ artifactRoot: root, budgetLedger: ledger, runId: randomUUID(), lane: "offline", scenario: "baseline" });
  const descendantFile = join(run.fixtureRoot, "descendant.pid");
  try {
    await run.spawn(process.execPath, ["-e", "console.log('ordinary command')"]);
    await assert.rejects(run.spawn(process.execPath, ["-e",
      "const c=require('child_process').spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'}); require('fs').writeFileSync(process.argv[1],String(c.pid)); setInterval(()=>{},1000)", descendantFile],
    { timeoutMs: 1000 }), { code: "PROCESS_TIMEOUT" });
    const pid = Number(await readFile(descendantFile, "utf8"));
    assert.throws(() => process.kill(pid, 0), { code: "ESRCH" });
    const cleanup = await run.cleanup();
    assert.equal(cleanup.owned_resources.length, 2);
    assert.ok(cleanup.owned_resources.every((entry) => entry.closed && entry.absent));
    if (process.platform === "win32") assert.ok(cleanup.owned_resources[1].cancelled_tree_pids.includes(pid));
    const events = (await readFile(join(run.runRoot, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
    assert.ok(events.every((entry, index) => entry.sequence === index && entry.elapsed_ms >= (events[index - 1]?.elapsed_ms ?? 0)));
  } finally { await run.cleanup(); }
});

test("spawn failures remain explicit and logs redact synthetic credentials", async (t) => {
  const root = await fixture(t);
  const ledger = join(root, "live-budget.json");
  await writeFile(ledger, '{"reserved":0,"sent":0}\n');
  const { createOwnedRun } = await runner();
  const run = await createOwnedRun({ artifactRoot: root, budgetLedger: ledger, runId: randomUUID(), lane: "offline", scenario: "baseline" });
  try {
    await assert.rejects(run.spawn("missing-workflow-executable", []), { code: "ENOENT" });
    const result = await run.spawn(process.execPath, ["-e", "console.log('Bearer synthetic-credential-sentinel'); console.error('synthetic-credential-sentinel')"]);
    assert.doesNotMatch(result.stdout + result.stderr, /synthetic-credential-sentinel/);
    const cleanup = await run.cleanup();
    for (const entry of cleanup.owned_resources) {
      assert.doesNotMatch(await readFile(join(run.runRoot, entry.log_path), "utf8"), /synthetic-credential-sentinel/);
    }
    assert.doesNotMatch(await readFile(join(run.runRoot, "cleanup.json"), "utf8"), /synthetic-credential-sentinel/);
    assert.doesNotMatch(await readFile(join(run.runRoot, "events.jsonl"), "utf8"), /synthetic-credential-sentinel/);
  } finally { await run.cleanup(); }
});
