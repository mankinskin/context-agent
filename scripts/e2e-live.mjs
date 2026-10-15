import assert from "node:assert/strict";
import { createHash, randomUUID } from "node:crypto";
import { copyFile, cp, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { artifacts, createOwnedRun, ensurePortAvailable, fileHash, freePort, mediaProjects, provisionFixtures, waitForReady } from "./e2e-runner.mjs";
import { snapshotSessionStores } from "./e2e-native.mjs";
import { failure } from "./e2e-lifecycle.mjs";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const readJson = async path => JSON.parse(await readFile(path, "utf8"));
const id = "10000000-0000-0000-0000-000000000001";

export function verifyLiveNative(manifest, transcript, providerEvents) {
  assert.equal(manifest.source, "context-agent");
  assert.equal(transcript.session_id, manifest.session_id);
  const events = transcript.turns.map((turn, index) => {
    const event = JSON.parse(turn.content);
    assert.equal(turn.sequence, index);
    assert.equal(event.sequence, index);
    assert.equal(turn.event_meta.event_id, event.event_id);
    assert.equal(turn.event_meta.parent_event_id ?? null, index ? transcript.turns[index - 1].event_meta.event_id : null);
    return event;
  });
  assert.deepEqual(events.slice(0, 3).map(event => event.evidence_kind), ["chat_message", "prompt_assembly", "model_request"]);
  assert.equal(events.at(-1).evidence_kind, "model_response");
  assert.equal(new Set(events.map(event => event.event_id)).size, events.length);
  const tools = events.filter(event => event.evidence_kind === "tool_call");
  assert.ok(tools.length > 0, "native actual tool records required");
  for (const tool of tools) {
    assert.equal(tool.message.role, "tool");
    assert.equal(tool.tool_call_id, tool.message.tool_calls[0].tool_call_id);
    assert.match(tool.message.tool_calls[0].summary, /workflow-proof-29ad8d96/);
    const requested = providerEvents.find(event => event.kind === "tool_requested" && event.tool_id === tool.tool_call_id);
    const returned = providerEvents.find(event => event.kind === "tool_result_submitted" && event.tool_id === tool.tool_call_id);
    assert.ok(requested && returned && requested.sequence < returned.sequence, "native and actual provider tool identity/order must match");
  }
  return { session_id: manifest.session_id, tool_ids: tools.map(tool => tool.tool_call_id),
    event_ids: events.map(event => event.event_id), usage: events.at(-1).message.usage ?? null };
}

export async function verifyLiveArtifacts(root) {
  const manifest = await readJson(join(root, "manifest.json"));
  const cleanup = await readJson(join(root, "cleanup.json"));
  const before = await readJson(join(root, "ledger-before.json"));
  const after = await readJson(join(root, "ledger-after.json"));
  assert.equal(manifest.outcome, "passed");
  assert.ok(manifest.scenario_elapsed_ms <= 120_000);
  assert.ok(after.reserved <= 20 && after.reserved > before.reserved);
  assert.ok(after.sent > before.sent && after.sent <= after.reserved);
  assert.equal(manifest.provider_sends_delta, after.sent - before.sent);
  const events = (await readFile(join(root, "provider", "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
  for (const event of events) {
    assert.equal(event.model, "gpt-5-mini");
    assert.equal(event.max_output_tokens, 1024);
  }
  assert.ok(events.some(event => event.kind === "terminal_success"));
  const receipt = await readJson(join(root, "provider", "ticket-mcp-receipt.json"));
  assert.equal(receipt.exit_observed, true);
  assert.equal(receipt.service_joined, true);
  const observed = verifyLiveNative(await readJson(join(root, "native", "session.json")),
    await readJson(join(root, "native", "transcript.json")), events);
  assert.deepEqual(manifest.native_observed, observed);
  assert.equal(manifest.media_checks.length, 5);
  for (const check of manifest.media_checks) {
    const result = await readJson(join(root, check.result_file));
    assert.equal(result.stats.expected, 1);
    assert.equal(result.stats.unexpected + result.stats.skipped + result.stats.flaky, 0);
    assert.equal(result.config.projects[0].name, check.media.name);
    assert.equal(result.config.projects[0].metadata.hasTouch, check.media.hasTouch);
    assert.equal(result.config.projects[0].metadata.isMobile, check.media.isMobile);
    if (check.phase === "retained") assert.equal(check.ledger_before, check.ledger_after);
  }
  const productive = await readJson(join(root, "productive-session-stores.json"));
  assert.deepEqual(productive.after, productive.before);
  assert.equal(cleanup.remaining_resource_count, 0);
  assert.equal(cleanup.exit_join_result, "joined");
  assert.equal(cleanup.fixture_absent, true);
  assert.ok(cleanup.owned_resources.every(process => process.closed && process.absent));
  assert.ok(cleanup.owned_resources.some(process => process.pid === manifest.owned_pid));
  for (const item of [...manifest.static_assets, manifest.executable_identity, ...manifest.browser_artifacts, ...manifest.native_files]) {
    assert.equal(await fileHash(item.path), item.sha256);
  }
}

async function configure(run) {
  const workspace = process.platform === "win32" ? `\\\\?\\${run.manifest.store_root}` : run.manifest.store_root;
  const template = [
    "---", "model: gpt-5-mini", "tools:", "  - ticket_lookup", "---",
    `Use get_ticket exactly once with id ${id}, workspace ${JSON.stringify(workspace)}, view full.`,
    "Return the exact proof marker from its actual result. Never fabricate a result. No other tool or ticket is authorized.",
  ].join("\n");
  await writeFile(join(run.fixtureRoot, "proof.md"), template);
  const config = Object.entries({
    template_dir: run.fixtureRoot, template: "proof.md", model: "gpt-5-mini",
    file_root: run.fixtureRoot, ticket_workspace: run.manifest.store_root, ticket_mcp_command: "ticket-mcp",
  }).map(([key, value]) => `${key} = ${JSON.stringify(value)}`).join("\n");
  const path = join(run.fixtureRoot, "provider.toml");
  await writeFile(path, config);
  return path;
}

export async function runLive(options) {
  const run = await createOwnedRun({ artifactRoot: options.artifact_root, budgetLedger: options.budget_ledger,
    runId: randomUUID(), lane: "live", scenario: "spine" });
  console.log(`run_id=${run.manifest.run_id} artifacts=${run.runRoot}`);
  let error, start;
  try {
    const ledgerBefore = await readFile(options.budget_ledger);
    const ledger = JSON.parse(ledgerBefore);
    if (!Number.isInteger(ledger.reserved) || ledger.reserved > 18 || ledger.max_reservations !== 20
      || !Number.isInteger(ledger.sent) || ledger.sent > ledger.reserved) {
      throw failure("BUDGET_UNAVAILABLE", "invalid ledger or insufficient two-reservation spine headroom");
    }
    await writeFile(join(run.runRoot, "ledger-before.json"), ledgerBefore);
    run.manifest.revisions.context_agent = (await run.spawn("git", ["rev-parse", "HEAD"])).stdout.trim();
    run.manifest.scoped_dirty_state = (await run.spawn("git", ["status", "--short"])).stdout.trim();
    run.manifest.dirty_diff_sha256 = digest((await run.spawn("git", ["diff", "--binary", "HEAD"])).stdout);
    await provisionFixtures(run);
    const config = await configure(run);
    const builds = [
      await run.spawn("trunk", ["build", "--release"], { cwd: join(repo, "frontend"), role: "frontend-build" }),
      await run.spawn("cargo", ["build", "-p", "backend", "--locked"], { role: "backend-build" }),
    ];
    run.manifest.build = { commands: ["trunk build --release", "cargo build -p backend --locked"], pids: builds.map(build => build.pid) };
    const build = join(run.runRoot, "build");
    const staticRoot = join(build, "static");
    await mkdir(build);
    await cp(join(repo, "frontend", "dist"), staticRoot, { recursive: true });
    run.manifest.static_assets = await artifacts(staticRoot);
    run.manifest.static_identity = await fileHash(join(staticRoot, "index.html"));
    const executable = join(build, process.platform === "win32" ? "backend.exe" : "backend");
    await copyFile(join(repo, "target", "debug", process.platform === "win32" ? "backend.exe" : "backend"), executable);
    run.manifest.executable_identity = { path: executable, sha256: await fileHash(executable) };
    const port = await freePort();
    await ensurePortAvailable(port);
    const base = `http://127.0.0.1:${port}`;
    const nativeStore = join(run.fixtureRoot, "native-session-store");
    const productiveRoots = [join(repo, ".session"), join(repo, "..", ".workflow-tools", "session", "sessions")];
    const productiveBefore = await snapshotSessionStores(productiveRoots, "29ad8d96-9400-4627-a032-e02a3d8bd971", "context-agent");
    start = performance.now();
    const remaining = () => {
      const ms = Math.floor(110_000 - (performance.now() - start));
      if (ms <= 0) throw failure("SCENARIO_TIMEOUT", "live work deadline exceeded; cleanup reserved");
      return ms;
    };
    const server = run.startProcess(executable, [], { role: "backend", timeoutMs: remaining(), env: {
      PORT: String(port), STATIC_DIR: staticRoot, SESSION_STORE_ROOT: nativeStore,
      CONTEXT_AGENT_CONFIG: config, CONTEXT_AGENT_BUDGET_LEDGER: options.budget_ledger,
      CONTEXT_AGENT_EVIDENCE_ROOT: join(run.runRoot, "provider"),
      CONTEXT_AGENT_PROVIDER_SESSION_ID: ledger.route_session_id,
    } });
    run.manifest.base_url = base;
    run.manifest.owned_pid = server.record.pid;
    run.manifest.executable_identity.pid = server.record.pid;
    run.manifest.model = "gpt-5-mini";
    await run.persistManifest();
    const ready = performance.now();
    await waitForReady(base, { timeoutMs: Math.min(30_000, remaining()), processHandle: server });
    run.manifest.readiness_elapsed_ms = Math.round(performance.now() - ready);
    for (const asset of run.manifest.static_assets) {
      const relative = asset.path.slice(staticRoot.length + 1).replaceAll("\\", "/");
      const response = await fetch(`${base}/${relative}`, { signal: AbortSignal.timeout(Math.min(5_000, remaining())) });
      assert.equal(response.ok, true);
      asset.served_sha256 = digest(Buffer.from(await response.arrayBuffer()));
      assert.equal(asset.served_sha256, asset.sha256);
    }
    const cli = join(repo, "frontend", "e2e", "node_modules", "@playwright", "test", "cli.js");
    const origin = mediaProjects(options.media)[0];
    const media = [origin, ...mediaProjects("all").filter(project => project.name !== origin.name)];
    run.manifest.media_checks = [];
    run.manifest.browser_artifacts = [];
    for (let index = 0; index < media.length; index++) {
      const phase = index ? "retained" : "origin";
      const before = digest(await readFile(options.budget_ledger));
      const resultFile = `browser-${media[index].name}.json`;
      const output = join(run.runRoot, `browser-${media[index].name}`);
      await run.spawn(process.execPath, [cli, "test", "spine.spec.ts", "--reporter=list,json", "--output", output], {
        cwd: join(repo, "frontend", "e2e"), role: `browser-${media[index].name}`, timeoutMs: remaining(),
        env: { BASE_URL: base, E2E_MEDIA: JSON.stringify(media[index]), SPINE_PHASE: phase, PLAYWRIGHT_JSON_OUTPUT_FILE: join(run.runRoot, resultFile) },
      });
      const after = digest(await readFile(options.budget_ledger));
      if (index) assert.equal(after, before, "dependent inspection submitted provider");
      run.manifest.media_checks.push({ media: media[index], phase, result_file: resultFile, ledger_before: before, ledger_after: after });
      run.manifest.browser_artifacts.push(...await artifacts(output));
    }
    const sessions = await readdir(join(nativeStore, "sessions"));
    assert.equal(sessions.length, 1);
    await mkdir(join(run.runRoot, "native"));
    run.manifest.native_files = [];
    for (const name of ["session.json", "transcript.json"]) {
      const path = join(run.runRoot, "native", name);
      await copyFile(join(nativeStore, "sessions", sessions[0], name), path);
      run.manifest.native_files.push({ path, sha256: await fileHash(path) });
    }
    const provider = (await readFile(join(run.runRoot, "provider", "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
    run.manifest.native_observed = verifyLiveNative(await readJson(join(run.runRoot, "native", "session.json")), await readJson(join(run.runRoot, "native", "transcript.json")), provider);
    await writeFile(join(run.runRoot, "productive-session-stores.json"), JSON.stringify({
      before: productiveBefore, after: await snapshotSessionStores(productiveRoots, "29ad8d96-9400-4627-a032-e02a3d8bd971", "context-agent"),
    }, null, 2));
  } catch (cause) { error = cause; }
  try { await run.cleanup(); } catch (cause) { error = error ? new AggregateError([error, cause], `${error.message}; ${cause.message}`) : cause; }
  const after = await readFile(options.budget_ledger);
  await writeFile(join(run.runRoot, "ledger-after.json"), after);
  const before = await readJson(join(run.runRoot, "ledger-before.json"));
  run.manifest.provider_sends_delta = JSON.parse(after).sent - before.sent;
  run.manifest.scenario_elapsed_ms = start ? Math.round(performance.now() - start) : null;
  if (start && run.manifest.scenario_elapsed_ms > 120_000) error ??= failure("SCENARIO_TIMEOUT", "live inclusive deadline exceeded");
  run.manifest.end = new Date().toISOString();
  run.manifest.outcome = error ? "failed" : "passed";
  await run.event({ kind: "run_finished", outcome: run.manifest.outcome, code: error?.code ?? null });
  await run.persistManifest();
  if (!error) {
    try { await verifyLiveArtifacts(run.runRoot); } catch (cause) { error = cause; }
  }
  const assertions = [{ criterion: "APP-02", artifact_path: "native/transcript.json",
    expected: "real Copilot/MCP native result and five rendered media contexts, zero owned resources",
    observed: error?.message ?? run.manifest.native_observed, passed: !error }];
  await writeFile(join(run.runRoot, "assertions.json"), JSON.stringify(assertions, null, 2));
  if (error) {
    run.manifest.outcome = "failed";
    await run.persistManifest();
    throw error;
  }
}
