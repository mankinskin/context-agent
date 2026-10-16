import assert from "node:assert/strict";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { createOwnedRun, failure } from "./e2e-lifecycle.mjs";

const hash = value => createHash("sha256").update(value).digest("hex");
const readJson = async path => JSON.parse(await readFile(path, "utf8"));
export function sshForwardArgs(env = process.env) {
  if (env.E2E_SSH_AUTH_SOCK === undefined) return [];
  const socket = env.E2E_SSH_AUTH_SOCK;
  if (!socket || /[\r\n\0,]/.test(socket)) throw failure("INVALID_SSH_SOCKET", "E2E_SSH_AUTH_SOCK must name one SSH-agent socket");
  return ["--ssh", `default=${socket}`];
}
async function files(directory) {
  const result = [];
  for (const entry of await readdir(directory, { recursive: true, withFileTypes: true })) {
    if (entry.isFile()) {
      const path = join(entry.parentPath, entry.name);
      result.push({ path, sha256: hash(await readFile(path)) });
    }
  }
  return result.sort((a, b) => a.path.localeCompare(b.path));
}

export function browserResult(report, media, application) {
  assert.equal(report.stats.expected, 1);
  for (const key of ["unexpected", "skipped", "flaky"]) assert.equal(report.stats[key], 0);
  assert.equal(report.config.projects.length, 1);
  const project = report.config.projects[0];
  assert.equal(project.name, media.name);
  for (const key of ["viewport", "hasTouch", "isMobile"]) assert.deepEqual(project.metadata[key], media[key]);
  const specs = suite => [...(suite.specs ?? []), ...(suite.suites ?? []).flatMap(specs)];
  const tests = report.suites.flatMap(specs).flatMap(spec => spec.tests);
  assert.equal(tests.length, 1);
  assert.equal(tests[0].results.length, 1);
  const result = tests[0].results[0];
  assert.equal(result.status, "passed");
  const attachment = name => {
    const item = result.attachments.find(item => item.name === name);
    assert.ok(item?.body, `missing actual ${name} attachment`);
    return Buffer.from(item.body, "base64").toString("utf8");
  };
  const observed = JSON.parse(attachment(application === "template" ? "echo-observed" : "sessions-observed"));
  if (application === "template") assert.deepEqual(observed, {
    first: `Echoed: echo-${media.name}`, reloaded: `Echoed: echo-${media.name}-reloaded`,
  });
  else assert.equal(new Set(observed.session_ids).size, 2);
  return { observed, chromium: attachment("browser-version") };
}

export async function verifyContainerArtifacts(directory) {
  const runRoot = resolve(directory);
  const manifest = await readJson(join(runRoot, "manifest.json"));
  assert.equal(manifest.run_id, basename(runRoot));
  assert.equal(manifest.outcome, "passed");
  assert.equal(manifest.lane, "container");
  assert.match(manifest.base_url, /^http:\/\/127\.0\.0\.1:\d+$/);
  assert.match(manifest.container.image_id, /^sha256:[\da-f]{64}$/);
  const inspect = await readJson(join(runRoot, "container-inspect.json"));
  assert.equal(inspect.Image, manifest.container.image_id);
  assert.equal(inspect.Config.Labels["workflow.validation.run"], manifest.run_id);
  assert.equal(inspect.State.Running, false);
  assert.equal(inspect.State.ExitCode, manifest.container.exit_code);
  assert.ok(manifest.scenario_elapsed_ms > 0 && manifest.scenario_elapsed_ms <= 60_000);
  assert.deepEqual(browserResult(await readJson(join(runRoot, "browser-result.json")), manifest.media, manifest.application).observed, manifest.browser_observed);
  const endpoints = await readJson(join(runRoot, "endpoints.json"));
  assert.equal(endpoints.health.status, 200);
  assert.equal(endpoints.health.body, "ok");
  if (manifest.application === "template") {
    assert.equal(endpoints.echo.status, 200);
    assert.equal(endpoints.echo.body.message, `container-${manifest.run_id}`);
  } else {
    assert.equal(endpoints.sessions.status, 200);
    assert.deepEqual(endpoints.sessions.body.map(item => item.session_id).sort(), [...manifest.browser_observed.session_ids].sort());
  }
  assert.ok(manifest.browser_artifacts.some(item => item.path.endsWith("trace.zip")));
  assert.ok(manifest.browser_artifacts.some(item => item.path.endsWith(".png")));
  for (const item of [...manifest.browser_artifacts, ...manifest.static_assets, manifest.executable_identity]) {
    assert.equal(hash(await readFile(item.path)), item.sha256);
    if (manifest.static_assets.includes(item)) assert.equal(item.served_sha256, item.sha256);
  }
  const before = hash(await readFile(join(runRoot, "ledger-before.json")));
  const after = hash(await readFile(join(runRoot, "ledger-after.json")));
  assert.equal(before, after);
  assert.equal(before, manifest.ledger_before_sha256);
  assert.equal(after, manifest.ledger_after_sha256);
  const cleanup = await readJson(join(runRoot, "cleanup.json"));
  assert.equal(cleanup.exit_join_result, "joined");
  assert.equal(cleanup.remaining_resource_count, 0);
  assert.equal(cleanup.fixture_absent, true);
  assert.equal(cleanup.container_absent, true);
  assert.equal(cleanup.image_tag_absent, true);
  assert.ok(cleanup.owned_resources.every(item => item.closed && item.absent));
  const assertions = await readJson(join(runRoot, "assertions.json"));
  assert.equal(assertions.length, 1);
  assert.equal(assertions[0].passed, true);
  const events = (await readFile(join(runRoot, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
  events.forEach((event, index) => { assert.equal(event.sequence, index); assert.equal(event.run_id, manifest.run_id); });
  assert.equal(events.at(-1).kind, "run_finished");
  assert.equal(events.at(-1).outcome, "passed");
}

export async function runContainer(options, { root, media, application }) {
  const run = await createOwnedRun({ artifactRoot: options.artifact_root, budgetLedger: options.budget_ledger,
    runId: randomUUID(), lane: "container", scenario: options.scenario, cwd: root });
  const manifest = run.manifest;
  const label = "workflow.validation.run";
  const name = `validation-${manifest.run_id}`;
  const image = `validation-${application}:${manifest.run_id}`;
  let error, started, attemptedBuild = false, attemptedCreate = false, containerAbsent = false, imageAbsent = false;
  console.log(`run_id=${manifest.run_id} artifacts=${run.runRoot}`);
  manifest.application = application;
  manifest.media = media;
  manifest.container = { name, image };
  const docker = (args, timeoutMs = 15_000) => run.spawn("docker", args, { timeoutMs, role: `docker-${args[0]}` });
  async function inspect(kind, identity) {
    const child = run.startProcess("docker", [kind, "inspect", identity], { timeoutMs: 10_000, role: `inspect-${kind}` });
    const result = await child.completed;
    if (child.getError()) throw child.getError();
    if (child.timedOut()) throw failure("DOCKER_TIMEOUT", "cannot establish Docker resource state");
    if (child.record.exit_code === 0) return JSON.parse(result.stdout)[0];
    if (child.record.exit_code === 1 && /No such (?:container|image|object)/i.test(result.stderr)) return null;
    throw failure("DOCKER_INSPECT_FAILED", result.stderr);
  }
  try {
    const ledger = await readFile(options.budget_ledger);
    manifest.ledger_before_sha256 = hash(ledger);
    await writeFile(join(run.runRoot, "ledger-before.json"), ledger);
    await docker(["version", "--format", "{{.Server.Version}}"]);
    manifest.revision = (await run.spawn("git", ["rev-parse", "HEAD"])).stdout.trim();
    manifest.scoped_dirty_state = (await run.spawn("git", ["status", "--short"])).stdout.trim();
    manifest.source_identity = hash((await run.spawn("git", ["diff", "--binary", "HEAD"])).stdout + JSON.stringify(await files(join(root, "scripts"))));
    const base = process.env.E2E_BASE_IMAGE ?? "infrastructure-base:local";
    const baseInspect = await inspect("image", base);
    if (!baseInspect) throw failure("BASE_IMAGE_MISSING", `build the local infrastructure toolchain image first: ${base}; no private registry fallback`);
    manifest.container.base_image_id = baseInspect.Id;
    attemptedBuild = true;
    const sshArgs = sshForwardArgs();
    manifest.container.ssh_agent_forwarded = sshArgs.length > 0;
    await docker(["build", ...sshArgs, "--build-arg", `BASE_IMAGE=${base}`, "--label", `${label}=${manifest.run_id}`, "--tag", image, "."], 900_000);
    manifest.container.image_id = (await inspect("image", image)).Id;
    attemptedCreate = true;
    await docker(["create", "--name", name, "--label", `${label}=${manifest.run_id}`, "--publish", "127.0.0.1::8080",
      "--env", "PORT=8080", "--env", "STATIC_DIR=/app/frontend/dist", "--env", "SESSION_STORE_ROOT=/tmp/validation-sessions", image]);
    const build = join(run.runRoot, "build");
    await mkdir(build);
    await docker(["cp", `${name}:/app/frontend/dist`, join(build, "static")]);
    await docker(["cp", `${name}:/app/server`, join(build, "server")]);
    manifest.static_assets = await files(join(build, "static"));
    manifest.executable_identity = { path: join(build, "server"), sha256: hash(await readFile(join(build, "server"))) };
    started = performance.now();
    const remaining = () => {
      const ms = Math.floor(50_000 - (performance.now() - started));
      if (ms <= 0) throw failure("SCENARIO_TIMEOUT", "container work deadline exceeded; cleanup reserved");
      return ms;
    };
    await docker(["start", name], remaining());
    const running = await inspect("container", name);
    const port = running.NetworkSettings.Ports["8080/tcp"][0];
    assert.equal(port.HostIp, "127.0.0.1");
    manifest.base_url = `http://127.0.0.1:${port.HostPort}`;
    const endpoints = {};
    const readyStart = performance.now();
    while (true) {
      try {
        const response = await fetch(`${manifest.base_url}/health`, { signal: AbortSignal.timeout(Math.min(1000, remaining())) });
        endpoints.health = { status: response.status, body: await response.text() };
        if (response.status === 200 && endpoints.health.body === "ok") break;
      } catch (cause) { if (cause.code === "SCENARIO_TIMEOUT") throw cause; }
      if (performance.now() - readyStart > 30_000) throw failure("READINESS_TIMEOUT", "container health deadline exceeded");
      await new Promise(resolveWait => setTimeout(resolveWait, 100));
    }
    for (const item of manifest.static_assets) {
      const relative = item.path.slice(join(build, "static").length + 1).replaceAll("\\", "/");
      const response = await fetch(`${manifest.base_url}/${relative}`, { signal: AbortSignal.timeout(Math.min(5000, remaining())) });
      assert.equal(response.status, 200);
      item.served_sha256 = hash(Buffer.from(await response.arrayBuffer()));
      assert.equal(item.sha256, item.served_sha256);
    }
    const cli = join(root, "frontend", "e2e", "node_modules", "@playwright", "test", "cli.js");
    await run.spawn(process.execPath, [cli, "test", application === "template" ? "echo.spec.ts" : "baseline.spec.ts",
      "--config", "playwright.config.ts", "--reporter=list,json", "--output", join(run.runRoot, "browser")], {
      cwd: join(root, "frontend", "e2e"), role: "browser", timeoutMs: remaining(), env: {
        BASE_URL: manifest.base_url, E2E_MEDIA: JSON.stringify(media), PLAYWRIGHT_JSON_OUTPUT_FILE: join(run.runRoot, "browser-result.json"),
      },
    });
    const report = await readJson(join(run.runRoot, "browser-result.json"));
    const browser = browserResult(report, media, application);
    manifest.browser_observed = browser.observed;
    manifest.tool_versions.chromium = browser.chromium;
    manifest.tool_versions.playwright = report.config.version;
    manifest.browser_artifacts = await files(join(run.runRoot, "browser"));
    const response = await fetch(`${manifest.base_url}/api/${application === "template" ? "echo" : "sessions"}`, {
      signal: AbortSignal.timeout(Math.min(5000, remaining())),
      ...(application === "template" ? { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ message: `container-${manifest.run_id}` }) } : {}),
    });
    endpoints[application === "template" ? "echo" : "sessions"] = { status: response.status, body: await response.json() };
    await writeFile(join(run.runRoot, "endpoints.json"), JSON.stringify(endpoints, null, 2));
  } catch (cause) { error = cause; }
  try {
    if (attemptedCreate) {
      const current = await inspect("container", name);
      if (current) {
        assert.equal(current.Config.Labels[label], manifest.run_id, "refuse foreign container cleanup");
        await docker(["logs", name]);
        if (current.State.Running) await docker(["stop", "--time", "2", name], 5000);
        const exited = await inspect("container", name);
        assert.equal(exited.State.Running, false);
        manifest.container.exit_code = exited.State.ExitCode;
        await writeFile(join(run.runRoot, "container-inspect.json"), JSON.stringify(exited, null, 2));
        await docker(["rm", name], 5000);
      }
      containerAbsent = await inspect("container", name) === null;
    } else containerAbsent = true;
    if (attemptedBuild) {
      const current = await inspect("image", image);
      if (current) {
        assert.equal(current.Config.Labels[label], manifest.run_id, "refuse foreign image cleanup");
        await docker(["image", "rm", image], 5000);
      }
      imageAbsent = await inspect("image", image) === null;
    } else imageAbsent = true;
  } catch (cause) { error = error ? new AggregateError([error, cause], `${error.message}; ${cause.message}`) : cause; }
  let cleanup;
  try { cleanup = await run.cleanup(); } catch (cause) { error = error ? new AggregateError([error, cause], `${error.message}; ${cause.message}`) : cause; }
  if (cleanup) {
    cleanup.container_absent = containerAbsent;
    cleanup.image_tag_absent = imageAbsent;
    cleanup.remaining_resource_count += Number(!containerAbsent) + Number(!imageAbsent);
    if (cleanup.remaining_resource_count) { cleanup.exit_join_result = "failed"; error ??= failure("CLEANUP_FAILED", "owned Docker resources remain"); }
    await writeFile(join(run.runRoot, "cleanup.json"), JSON.stringify(cleanup, null, 2));
  }
  const after = await readFile(options.budget_ledger);
  await writeFile(join(run.runRoot, "ledger-after.json"), after);
  manifest.ledger_after_sha256 = hash(after);
  if (manifest.ledger_after_sha256 !== manifest.ledger_before_sha256) error ??= failure("LEDGER_CHANGED", "container lane changed paid ledger");
  manifest.scenario_elapsed_ms = started ? Math.round(performance.now() - started) : null;
  if (started && manifest.scenario_elapsed_ms > 60_000) error ??= failure("SCENARIO_TIMEOUT", "inclusive container deadline exceeded");
  manifest.outcome = error ? "failed" : "passed";
  manifest.end = new Date().toISOString();
  if (error) manifest.failure = { code: error.code ?? "CONTAINER_FAILED", message: error.message };
  await writeFile(join(run.runRoot, "assertions.json"), JSON.stringify([{ criterion: "TPL-02", passed: !error }]));
  await run.event({ kind: "run_finished", outcome: manifest.outcome });
  await run.persistManifest();
  if (!error) {
    try { await verifyContainerArtifacts(run.runRoot); }
    catch (cause) { manifest.outcome = "failed"; await run.persistManifest(); error = cause; }
  }
  if (error) throw error;
}
