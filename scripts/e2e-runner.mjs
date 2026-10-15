import { createHash, randomUUID } from "node:crypto";
import { createServer } from "node:net";
import { copyFile, cp, mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createOwnedRun as ownedRun, failure } from "./e2e-lifecycle.mjs";
import { captureNativeBridge, snapshotSessionStores, verifyNativeBridge } from "./e2e-native.mjs";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
export const fileHash = async (path) => hash(await readFile(path));
export const createOwnedRun = (options) => ownedRun({ cwd: repositoryRoot, ...options });

const mediaMatrix = [
  { name: "desktop", viewport: { width: 1440, height: 900 }, isMobile: false, hasTouch: false, input: "mouse-keyboard", orientation: "landscape" },
  { name: "mobile-portrait", viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, input: "touch", orientation: "portrait" },
  { name: "mobile-landscape", viewport: { width: 844, height: 390 }, isMobile: true, hasTouch: true, input: "touch", orientation: "landscape" },
  { name: "tablet-portrait", viewport: { width: 768, height: 1024 }, isMobile: true, hasTouch: true, input: "touch", orientation: "portrait" },
  { name: "tablet-landscape", viewport: { width: 1024, height: 768 }, isMobile: true, hasTouch: true, input: "touch", orientation: "landscape" },
];

export function mediaProjects(media) {
  if (media === "all") return mediaMatrix;
  if (media === "desktop") return [mediaMatrix[0]];
  if (media === "mobile") return mediaMatrix.slice(1, 3);
  if (media === "tablet") return mediaMatrix.slice(3);
  throw failure("INVALID_MEDIA", `invalid media: ${media}`);
}

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help") return { help: true };
    if (!["--lane", "--scenario", "--media", "--artifact-root", "--budget-ledger", "--verify-artifacts"].includes(arg)) {
      throw failure("INVALID_ARGUMENT", `unknown argument: ${arg}`);
    }
    const value = argv[++index];
    if (!value || value.startsWith("--")) throw failure("INVALID_ARGUMENT", `invalid value for ${arg}`);
    const key = arg.slice(2).replaceAll("-", "_");
    if (key in options) throw failure("INVALID_ARGUMENT", `duplicate argument: ${arg}`);
    options[key] = value;
  }
  if (options.verify_artifacts) return options;
  options.media ??= "desktop";
  for (const name of ["lane", "scenario", "artifact_root", "budget_ledger"]) {
    if (!options[name]) throw failure("INVALID_ARGUMENT", `invalid: --${name.replaceAll("_", "-")} is required`);
  }
  if (!["offline", "live"].includes(options.lane)) throw failure("INVALID_ARGUMENT", `invalid lane: ${options.lane}`);
  if (!["baseline", "bridge", "spine"].includes(options.scenario)) throw failure("UNSUPPORTED_SCENARIO", `unsupported scenario: ${options.scenario}`);
  mediaProjects(options.media);
  if (options.lane === "live" && options.media === "all") throw failure("INVALID_MEDIA", "live requires one origin media project");
  if (options.lane === "live" && options.scenario !== "spine") throw failure("UNSUPPORTED_SCENARIO", "live requires scenario: spine");
  if (options.lane === "offline" && options.scenario === "spine") throw failure("UNSUPPORTED_SCENARIO", "spine is a live-only scenario");
  return options;
}

export async function ensurePortAvailable(port) {
  await new Promise((resolveProbe, reject) => {
    const probe = createServer();
    probe.once("error", (error) => reject(error.code === "EADDRINUSE"
      ? failure("PORT_IN_USE", `loopback port ${port} is occupied`) : error));
    probe.listen({ host: "127.0.0.1", port, exclusive: true }, () => probe.close((error) => error ? reject(error) : resolveProbe()));
  });
}

export async function waitForReady(baseUrl, { timeoutMs = 30_000, intervalMs = 200, processHandle } = {}) {
  const deadline = performance.now() + timeoutMs;
  let lastError;
  while (performance.now() < deadline) {
    if (processHandle?.record.closed) throw failure("SERVER_EXITED", "owned backend exited before readiness");
    try {
      const remaining = Math.max(1, Math.ceil(deadline - performance.now()));
      const response = await fetch(`${baseUrl}/health`, { signal: AbortSignal.timeout(Math.min(1000, remaining)) });
      if (response.ok && await response.text() === "ok") return;
      lastError = new Error(`health returned ${response.status}`);
    } catch (error) { lastError = error; }
    await new Promise((resolveWait) => setTimeout(resolveWait, Math.min(intervalMs, Math.max(0, deadline - performance.now()))));
  }
  throw failure("READINESS_TIMEOUT", `readiness deadline exceeded: ${lastError?.message ?? "no response"}`);
}

export async function freePort() {
  return new Promise((resolvePort, reject) => {
    const probe = createServer();
    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const port = probe.address().port;
      probe.close((error) => error ? reject(error) : resolvePort(port));
    });
  });
}

export async function provisionFixtures(run) {
  const body = join(run.fixtureRoot, "synthetic-body.md");
  await mkdir(run.manifest.store_root);
  await writeFile(body, "workflow-proof-29ad8d96\n");
  const id = "10000000-0000-0000-0000-000000000001";
  await run.spawn("ticket.exe", ["create", "--id", id, "--workspace", run.manifest.store_root,
    "--type", "task", "--title", "Workflow proof", "--body-file", body], { timeoutMs: 30_000 });
  const result = await run.spawn("ticket.exe", ["get", id, "--workspace", run.manifest.store_root, "--json"], { timeoutMs: 30_000 });
  const ticket = JSON.parse(result.stdout).payload.ticket;
  const description = await run.spawn("ticket.exe", ["describe", id, "--workspace", run.manifest.store_root, "--view", "full", "--json"], { timeoutMs: 30_000 });
  if (ticket.id !== id || !JSON.stringify(JSON.parse(description.stdout)).includes("workflow-proof-29ad8d96")) {
    throw failure("FIXTURE_INVALID", "ticket fixture ID/proof read-back differs");
  }
  const gitRoot = join(run.fixtureRoot, "git");
  await mkdir(gitRoot);
  await writeFile(join(gitRoot, "README.md"), "workflow-proof-29ad8d96\n");
  await run.spawn("git", ["init"], { cwd: gitRoot, timeoutMs: 30_000 });
  await run.spawn("git", ["add", "README.md"], { cwd: gitRoot, timeoutMs: 30_000 });
  run.manifest.fixture_proof = { ticket_id: ticket.id, marker: "workflow-proof-29ad8d96", git_root: gitRoot };
  await run.event({ kind: "fixtures_verified", ...run.manifest.fixture_proof });
}

export async function artifacts(root) {
  const entries = await readdir(root, { recursive: true, withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (entry.isFile()) {
      const path = join(entry.parentPath, entry.name);
      files.push({ path, sha256: await fileHash(path), bytes: (await stat(path)).size });
    }
  }
  return files.sort((a, b) => a.path.localeCompare(b.path));
}

async function verifyArtifacts(root) {
  const runRoot = resolve(root);
  try {
    const read = async (name) => JSON.parse(await readFile(join(runRoot, name), "utf8"));
    const [manifest, assertions, cleanup] = await Promise.all(["manifest.json", "assertions.json", "cleanup.json"].map(read));
    if (manifest.lane === "live") return (await import("./e2e-live.mjs")).verifyLiveArtifacts(runRoot);
    const events = (await readFile(join(runRoot, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
    const requireEvidence = (condition, message) => { if (!condition) throw failure("ARTIFACT_INVALID", message); };
    requireEvidence(manifest.outcome === "passed" && manifest.run_id === basename(runRoot), "failed or mismatched run identity");
    requireEvidence(["baseline", "bridge"].includes(manifest.scenario) && manifest.lane === "offline", "missing lane/scenario");
    requireEvidence(manifest.browser?.base_url === manifest.base_url && /^http:\/\/127\.0\.0\.1:\d+$/.test(manifest.base_url), "missing URL identity");
    requireEvidence(manifest.owned_pid === manifest.executable_identity?.pid && manifest.owned_pid > 0, "missing executable PID identity");
    requireEvidence(manifest.executable_identity?.sha256?.length === 64 && manifest.static_identity?.length === 64, "missing build identity");
    requireEvidence(manifest.revisions?.context_agent && manifest.dirty_diff_sha256?.length === 64 && manifest.build?.commands?.length === 2, "missing revision/build evidence");
    requireEvidence(manifest.tool_versions?.playwright && manifest.tool_versions?.chromium, "missing browser versions");
    requireEvidence(manifest.fixture_proof?.marker === "workflow-proof-29ad8d96", "missing fixture read-back");
    requireEvidence(manifest.scenario_elapsed_ms <= 60_000 && manifest.readiness_elapsed_ms <= 30_000, "deadline exceeded");
    requireEvidence(Array.isArray(assertions) && assertions.length === (manifest.scenario === "bridge" ? 5 : 4) && assertions.every((item) => item.passed === true)
      && ["RUN-01", "RUN-02", "RUN-03", "RUN-04"].every((id) => assertions.some((item) => item.criterion === id)), "missing passing criterion evidence");
    requireEvidence(cleanup.exit_join_result === "joined" && cleanup.remaining_resource_count === 0 && cleanup.fixture_absent === true
      && cleanup.owned_resources.some((entry) => entry.pid === manifest.owned_pid && entry.closed && entry.absent), "missing observed backend cleanup");
    requireEvidence(cleanup.owned_resources.every((entry) => entry.closed && entry.absent), "unjoined owned process");
    requireEvidence(events.length > 4 && events.every((entry, index) => entry.sequence === index && entry.run_id === manifest.run_id
      && entry.elapsed_ms >= (events[index - 1]?.elapsed_ms ?? 0)), "invalid lifecycle sequence");
    requireEvidence(events.at(-1).kind === "run_finished" && events.at(-1).outcome === "passed", "missing terminal lifecycle evidence");
    requireEvidence(manifest.ledger_before_sha256 === manifest.ledger_after_sha256 && manifest.provider_sends_delta === 0, "ledger changed");
    requireEvidence(await fileHash(join(runRoot, "ledger-before.json")) === manifest.ledger_before_sha256
      && await fileHash(join(runRoot, "ledger-after.json")) === manifest.ledger_after_sha256, "missing actual ledger snapshots");
    await stat(cleanup.preserved_ledger_path);
    const browserResult = await read("browser-result.json");
    requireEvidence(browserResult.stats.expected === 1 && browserResult.stats.unexpected === 0
      && browserResult.stats.skipped === 0 && browserResult.stats.flaky === 0, "browser assertions did not complete");
    const screenshots = manifest.scenario === "bridge" ? ["bridge-before.png", "bridge-after.png"]
      : manifest.media ? ["baseline-responsive.png"] : ["baseline-desktop.png", "baseline-narrow.png"];
    if (manifest.media) {
      requireEvidence(browserResult.config.projects.length === 1
        && browserResult.config.projects[0].name === manifest.media.name
        && JSON.stringify(browserResult.config.projects[0].metadata.viewport) === JSON.stringify(manifest.media.viewport)
        && browserResult.config.projects[0].metadata.hasTouch === manifest.media.hasTouch
        && browserResult.config.projects[0].metadata.isMobile === manifest.media.isMobile, "missing genuine media context");
    }
    requireEvidence(manifest.browser.artifacts.some((item) => item.path.endsWith("trace.zip"))
      && screenshots.every((name) => manifest.browser.artifacts.some((item) => item.path.endsWith(name))), "missing browser evidence");
    const nativeFiles = [];
    if (manifest.scenario === "bridge") {
      requireEvidence(assertions.some((item) => item.criterion === "APP-01"), "missing native bridge criterion");
      const bridge = manifest.bridge;
      requireEvidence(bridge?.before_pid > 0 && bridge.after_pid > 0 && bridge.before_pid !== bridge.after_pid, "missing real backend restart");
      requireEvidence(cleanup.owned_resources.some((entry) => entry.pid === bridge.after_pid && entry.closed && entry.absent), "restarted backend was not joined");
      for (const phase of ["before", "after"]) {
        const record = bridge[phase];
        const observed = verifyNativeBridge(await read(`native-${phase}/session.json`), await read(`native-${phase}/transcript.json`), bridge.store_root);
        requireEvidence(JSON.stringify(observed) === JSON.stringify(record.observed), "native read-back differs");
        nativeFiles.push(...record.files);
      }
      requireEvidence(JSON.stringify(bridge.before.observed) === JSON.stringify(bridge.after.observed), "restart changed durable identity or history");
      const beforeResult = await read("browser-before-result.json");
      requireEvidence(beforeResult.stats.expected === 1 && beforeResult.stats.unexpected === 0 && beforeResult.stats.skipped === 0 && beforeResult.stats.flaky === 0, "pre-restart browser did not pass");
      requireEvidence(events.some((entry) => entry.kind === "backend_restarted" && entry.before_pid === bridge.before_pid && entry.after_pid === bridge.after_pid), "missing restart lifecycle evidence");
      const stores = await read("productive-session-stores.json");
      requireEvidence(JSON.stringify(stores.before) === JSON.stringify(stores.after), "productive session store changed");
      requireEvidence(stores.excluded_session_id === "29ad8d96-9400-4627-a032-e02a3d8bd971"
        && stores.source_filter === "context-agent" && stores.before.length === 2, "missing productive store scope");
    }
    for (const item of [...manifest.browser.artifacts, ...manifest.static_assets, manifest.executable_identity, ...nativeFiles]) {
      requireEvidence(item.path.startsWith(`${runRoot}\\`) || item.path.startsWith(`${runRoot}/`), "artifact outside owned run");
      requireEvidence(await fileHash(item.path) === item.sha256, `artifact hash differs: ${item.path}`);
    }
    for (const process of cleanup.owned_resources) {
      requireEvidence(events.some((entry) => entry.kind === "process_started" && entry.pid === process.pid)
        && events.some((entry) => entry.kind === "process_exited" && entry.pid === process.pid && entry.closed), "missing process lifecycle evidence");
      await stat(join(runRoot, process.log_path));
    }
    requireEvidence(manifest.static_assets.length > 1 && manifest.static_assets.every((item) => item.served_sha256 === item.sha256), "served build differs");
  } catch (error) {
    if (error.code === "ARTIFACT_INVALID") throw error;
    throw failure("ARTIFACT_INVALID", `missing or invalid artifact evidence: ${error.message}`);
  }
}

async function runBaseline(options) {
  const run = await createOwnedRun({ artifactRoot: options.artifact_root, budgetLedger: options.budget_ledger,
    runId: randomUUID(), lane: options.lane, scenario: options.scenario });
  console.log(`run_id=${run.manifest.run_id} artifacts=${run.runRoot}`);
  let error, scenarioStart, deadline;
  run.manifest.media = options.project;
  try {
    const ledgerBefore = await readFile(options.budget_ledger);
    const ledger = JSON.parse(ledgerBefore);
    if (!Number.isInteger(ledger.reserved) || !Number.isInteger(ledger.sent) || ledger.reserved < ledger.sent
      || ledger.reserved > 20 || ledger.sent < 0) throw failure("LEDGER_INVALID", "invalid durable budget counters");
    run.manifest.ledger_before_sha256 = hash(ledgerBefore);
    await writeFile(join(run.runRoot, "ledger-before.json"), ledgerBefore);
    run.manifest.ledger_counts = { reserved: ledger.reserved, sent: ledger.sent };
    run.manifest.revisions.context_agent = (await run.spawn("git", ["rev-parse", "HEAD"])).stdout.trim();
    run.manifest.scoped_dirty_state = (await run.spawn("git", ["status", "--short"])).stdout.trim();
    const diff = await run.spawn("git", ["diff", "--binary", "HEAD"]);
    run.manifest.dirty_diff_sha256 = hash(diff.stdout + JSON.stringify(await artifacts(join(repositoryRoot, "scripts"))));
    const productiveRoots = [join(repositoryRoot, ".session"), join(repositoryRoot, "..", ".workflow-tools", "session", "sessions")];
    const excludedSessionId = "29ad8d96-9400-4627-a032-e02a3d8bd971";
    const productiveBefore = options.scenario === "bridge"
      ? await snapshotSessionStores(productiveRoots, excludedSessionId, "context-agent") : null;
    await provisionFixtures(run);
    const frontendBuild = await run.spawn("trunk", ["build", "--release"], { cwd: join(repositoryRoot, "frontend"), role: "frontend-build" });
    const backendBuild = await run.spawn("cargo", ["build", "-p", "backend"], { role: "backend-build" });
    run.manifest.build = { commands: ["trunk build --release", "cargo build -p backend"], pids: [frontendBuild.pid, backendBuild.pid] };
    const buildRoot = join(run.runRoot, "build");
    const staticRoot = join(buildRoot, "static");
    await mkdir(buildRoot);
    await cp(join(repositoryRoot, "frontend", "dist"), staticRoot, { recursive: true });
    run.manifest.static_assets = await artifacts(staticRoot);
    run.manifest.static_identity = await fileHash(join(staticRoot, "index.html"));
    const executableName = process.platform === "win32" ? "backend.exe" : "backend";
    const executable = join(buildRoot, executableName);
    await copyFile(join(repositoryRoot, "target", "debug", executableName), executable);
    run.manifest.executable_identity = { path: executable, sha256: await fileHash(executable) };
    const cli = join(repositoryRoot, "frontend", "e2e", "node_modules", "@playwright", "test", "cli.js");
    run.manifest.tool_versions.playwright = (await run.spawn(process.execPath, [cli, "--version"])).stdout.trim();
    const port = await freePort();
    await ensurePortAvailable(port);
    const baseUrl = `http://127.0.0.1:${port}`;
    run.manifest.base_url = baseUrl;
    scenarioStart = performance.now();
    deadline = scenarioStart + 60_000;
    const remaining = () => {
      const ms = Math.floor(deadline - performance.now() - 10_000);
      if (ms <= 0) throw failure("SCENARIO_TIMEOUT", "offline work deadline exceeded; cleanup reserved");
      return ms;
    };
    const nativeStore = join(run.fixtureRoot, "native-session-store");
    const startBackend = () => run.startProcess(executable, [], { role: "backend", env: { PORT: String(port), STATIC_DIR: staticRoot, SESSION_STORE_ROOT: nativeStore }, timeoutMs: remaining() });
    const server = startBackend();
    run.manifest.owned_pid = server.record.pid;
    run.manifest.executable_identity.pid = server.record.pid;
    await run.persistManifest();
    const readinessStart = performance.now();
    await waitForReady(baseUrl, { timeoutMs: Math.min(30_000, remaining()), processHandle: server });
    run.manifest.readiness_elapsed_ms = Math.round(performance.now() - readinessStart);
    for (const asset of run.manifest.static_assets) {
      const relative = asset.path.slice(staticRoot.length + 1).split("\\").join("/");
      const response = await fetch(`${baseUrl}/${relative}`, { signal: AbortSignal.timeout(Math.min(5000, remaining())) });
      if (!response.ok) throw failure("BUILD_MISMATCH", `asset returned ${response.status}: ${relative}`);
      asset.served_sha256 = hash(Buffer.from(await response.arrayBuffer()));
      if (asset.sha256 !== asset.served_sha256 || server.record.closed) throw failure("BUILD_MISMATCH", "served asset or owned server differs");
    }
    await run.event({ kind: "server_ready", pid: server.record.pid, base_url: baseUrl });
    const runBrowser = (phase) => run.spawn(process.execPath, [cli, "test", options.scenario === "bridge" ? "bridge.spec.ts" : "baseline.spec.ts", "--config", "playwright.config.ts", "--reporter=list,json", "--output", join(run.runRoot, phase === "before" ? "browser-before" : "browser")], {
      cwd: join(repositoryRoot, "frontend", "e2e"), env: { BASE_URL: baseUrl, E2E_MEDIA: JSON.stringify(options.project), BRIDGE_PHASE: phase, PLAYWRIGHT_JSON_OUTPUT_FILE: join(run.runRoot, phase === "before" ? "browser-before-result.json" : "browser-result.json") }, timeoutMs: remaining(), role: `browser-${phase}`,
    });
    if (options.scenario === "bridge") {
      await runBrowser("before");
      const before = await captureNativeBridge(nativeStore, run.runRoot, "before");
      await server.stop();
      await server.completed;
      if (!server.record.closed || !server.record.absent) throw failure("RESTART_NOT_JOINED", "original backend did not exit");
      await ensurePortAvailable(port);
      const restarted = startBackend();
      await waitForReady(baseUrl, { timeoutMs: Math.min(30_000, remaining()), processHandle: restarted });
      run.manifest.bridge = { before_pid: server.record.pid, after_pid: restarted.record.pid, store_root: nativeStore, before };
      await run.event({ kind: "backend_restarted", before_pid: server.record.pid, after_pid: restarted.record.pid });
    }
    const browser = await runBrowser(options.scenario === "bridge" ? "after" : "baseline");
    run.manifest.browser = { pid: browser.pid, base_url: baseUrl, artifacts: await artifacts(join(run.runRoot, "browser")) };
    if (options.scenario === "bridge") {
      run.manifest.browser.artifacts.push(...await artifacts(join(run.runRoot, "browser-before")));
      run.manifest.bridge.after = await captureNativeBridge(nativeStore, run.runRoot, "after");
      const after = await snapshotSessionStores(productiveRoots, excludedSessionId, "context-agent");
      await writeFile(join(run.runRoot, "productive-session-stores.json"), `${JSON.stringify({
        excluded_session_id: excludedSessionId, source_filter: "context-agent", before: productiveBefore, after,
        exclusion_reason: "Only productive application manifests/transcripts are compared. Agent capture records and store indexes have independent concurrent writers; no whole-workspace no-write claim is made."
      }, null, 2)}\n`);
      if (JSON.stringify(productiveBefore) !== JSON.stringify(after)) throw failure("PRODUCTIVE_STORE_CHANGED", "productive session store fingerprint changed; inspect concurrent writers before proceeding");
    }
    const results = JSON.parse(await readFile(join(run.runRoot, "browser-result.json"), "utf8"));
    const attachment = results.suites[0].specs[0].tests[0].results[0].attachments.find((item) => item.name === "browser-version");
    run.manifest.tool_versions.chromium = attachment.body
      ? Buffer.from(attachment.body, "base64").toString() : (await readFile(attachment.path, "utf8"));
    const ledgerAfter = await readFile(options.budget_ledger);
    await writeFile(join(run.runRoot, "ledger-after.json"), ledgerAfter);
    run.manifest.ledger_after_sha256 = hash(ledgerAfter);
    if (run.manifest.ledger_before_sha256 !== run.manifest.ledger_after_sha256) throw failure("LEDGER_CHANGED", "offline changed durable ledger");
    run.manifest.provider_sends_delta = 0;
    remaining();
  } catch (cause) { error = cause; }
  try { await run.cleanup(); } catch (cause) { error = error ? new AggregateError([error, cause], `${error.message}; ${cause.message}`) : cause; }
  run.manifest.scenario_elapsed_ms = scenarioStart ? Math.round(performance.now() - scenarioStart) : null;
  if (deadline && performance.now() > deadline) error ??= failure("SCENARIO_TIMEOUT", "offline cleanup deadline exceeded");
  run.manifest.end = new Date().toISOString();
  run.manifest.outcome = error ? "failed" : "passed";
  const assertions = error ? [{ criterion: "runner", observed: error.message, passed: false }] : [
    { criterion: "RUN-01", artifact_path: "browser", expected: options.scenario === "bridge" ? "UI submission and restart restoration" : "desktop/narrow baseline", observed: run.manifest.browser, passed: true },
    { criterion: "RUN-02", artifact_path: "manifest.json", expected: "owned build/server/browser", observed: run.manifest.base_url, passed: true },
    { criterion: "RUN-03", artifact_path: "cleanup.json", expected: 0, observed: 0, passed: true },
    { criterion: "RUN-04", artifact_path: "manifest.json", expected: "unchanged ledger", observed: run.manifest.ledger_after_sha256, passed: true },
  ];
  if (!error && options.scenario === "bridge") assertions.push({ criterion: "APP-01", artifact_path: "native-after/transcript.json", expected: "four genuine ordered native events retained after restart", observed: run.manifest.bridge.after.observed, passed: true });
  await writeFile(join(run.runRoot, "assertions.json"), `${JSON.stringify(assertions, null, 2)}\n`);
  await run.event({ kind: "run_finished", outcome: run.manifest.outcome, code: error?.code ?? null, ...run.manifest.ledger_counts, provider_sends_delta: run.manifest.provider_sends_delta });
  await run.persistManifest();
  if (error) throw error;
  try { await verifyArtifacts(run.runRoot); }
  catch (cause) {
    run.manifest.outcome = "failed";
    await writeFile(join(run.runRoot, "assertions.json"), `${JSON.stringify([{ criterion: "artifact-verification", observed: cause.message, passed: false }])}\n`);
    await run.event({ kind: "run_finished", outcome: "failed", code: cause.code });
    await run.persistManifest();
    throw cause;
  }
}

async function main() {
  try {
    const options = parseArgs(process.argv.slice(2));
    if (options.help) {
      console.log("Usage: --lane offline|live --scenario baseline|bridge|spine --media all|desktop|mobile|tablet --artifact-root <path> --budget-ledger <path>\n--verify-artifacts <run-path> reads prior evidence only; --help\nMedia defaults to desktop for existing callers. Offline all runs five isolated cases sequentially; live rejects all, submits once at origin then inspects retained results without submission.");
    } else if (options.verify_artifacts) {
      await verifyArtifacts(options.verify_artifacts);
      console.log("artifact verification passed");
    } else {
      if (options.lane === "live") {
        await (await import("./e2e-live.mjs")).runLive(options);
      } else {
        for (const project of mediaProjects(options.media)) await runBaseline({ ...options, project });
      }
      console.log(`${options.lane} ${options.scenario} passed`);
    }
  } catch (error) {
    console.error(`${error.code ?? "RUNNER_FAILED"}: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch(error => {
    console.error(`${error.code ?? "RUNNER_FAILED"}: ${error.message}`);
    process.exitCode = 1;
  });
}
