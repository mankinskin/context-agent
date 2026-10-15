import { spawn } from "node:child_process";
import { appendFile, mkdir, rm, stat, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";

export function failure(code, message) {
  return Object.assign(new Error(message), { code });
}

export function safeEnvironment(extra = {}) {
  const allowed = ["PATH", "SystemRoot", "WINDIR", "ComSpec", "TEMP", "TMP", "HOME", "USERPROFILE", "LOCALAPPDATA", "APPDATA", "ProgramFiles", "ProgramFiles(x86)", "NUMBER_OF_PROCESSORS"];
  const environment = Object.fromEntries(allowed.flatMap((name) => process.env[name] ? [[name, process.env[name]]] : []));
  for (const [name, value] of Object.entries(extra)) {
    if (!/(?:token|secret|api.?key|authorization|credential)/i.test(name)) environment[name] = value;
  }
  return environment;
}

function redacted(text) {
  return text.replace(/(?:Bearer\s+\S+|(?:gh[pousr]_|github_pat_)[\w]+|synthetic-credential-sentinel)/gi, "[REDACTED]")
    .replace(/((?:authorization|api[_-]?key|access[_-]?token|secret)\s*[:=]\s*)[^\s,"]+/gi, "$1[REDACTED]");
}

function alive(pid) {
  try { process.kill(pid, 0); return true; }
  catch (error) { if (error.code === "ESRCH") return false; throw error; }
}

async function bounded(promise, ms, code) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(failure(code, `deadline exceeded after ${ms}ms`)), ms);
    })]);
  } finally { clearTimeout(timer); }
}

async function stopTree(pid) {
  if (!alive(pid)) return [];
  if (process.platform !== "win32") {
    try { process.kill(-pid, "SIGKILL"); }
    catch (error) { if (error.code !== "ESRCH") throw error; }
    return [pid];
  }
  const command = `$ErrorActionPreference='Stop'; $all=@(Get-CimInstance Win32_Process); $ids=[System.Collections.Generic.List[int]]::new(); function StopOwned([int]$id) { foreach($p in $all) { if($p.ParentProcessId -eq $id) { StopOwned $p.ProcessId } }; $ids.Add($id); $p=Get-Process -Id $id -ErrorAction SilentlyContinue; if($p) { Stop-Process -Id $id -Force } }; StopOwned ${pid}; $deadline=[DateTime]::UtcNow.AddSeconds(2); do { $live=@($ids | Where-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue }); if($live.Count) { Start-Sleep -Milliseconds 20 } } while($live.Count -and [DateTime]::UtcNow -lt $deadline); if($live.Count) { throw "owned PIDs $live still alive" }; ConvertTo-Json -Compress -InputObject @($ids.ToArray())`;
  const killer = spawn("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", command], {
    env: safeEnvironment(), windowsHide: true, stdio: ["ignore", "pipe", "pipe"],
  });
  let diagnostic = "";
  let output = "";
  killer.stdout.on("data", (chunk) => { output = (output + chunk).slice(-20_000); });
  killer.stderr.on("data", (chunk) => { diagnostic = (diagnostic + chunk).slice(-2000); });
  const done = new Promise((resolveDone, reject) => {
    killer.once("error", reject);
    killer.once("close", (code) => code === 0 ? resolveDone() : reject(failure("CANCEL_FAILED", redacted(diagnostic))));
  });
  try { await bounded(done, 5000, "CANCEL_TIMEOUT"); return JSON.parse(output); }
  finally { if (killer.exitCode === null && killer.signalCode === null) killer.kill(); }
}

export async function createOwnedRun({ artifactRoot, budgetLedger, runId, lane, scenario, cwd = process.cwd() }) {
  if (!/^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(runId)) throw failure("INVALID_RUN_ID", "run ID must be a UUID");
  const root = resolve(artifactRoot);
  const runRoot = join(root, runId);
  const fixtureRoot = join(runRoot, "fixtures");
  await mkdir(root, { recursive: true });
  await mkdir(runRoot);
  await mkdir(fixtureRoot);
  await mkdir(join(runRoot, "browser"));
  await mkdir(join(runRoot, "logs"));
  const start = performance.now();
  let sequence = 0;
  let pendingEvents = Promise.resolve();
  const event = (entry) => {
    const record = { ...entry, sequence: sequence++, run_id: runId, elapsed_ms: Math.round(performance.now() - start) };
    pendingEvents = pendingEvents.then(() => appendFile(join(runRoot, "events.jsonl"), `${JSON.stringify(record)}\n`));
    return pendingEvents;
  };
  const manifest = {
    run_id: runId, lane, scenario, revisions: {}, scoped_dirty_state: null, build: null,
    static_identity: null, executable_identity: null, base_url: null, owned_pid: null,
    fixture_root: fixtureRoot, store_root: join(fixtureRoot, "ticket-store"),
    tool_versions: { node: process.version }, model: null, ledger_path: resolve(budgetLedger),
    start: new Date().toISOString(), end: null, outcome: "running",
  };
  const persistManifest = () => writeFile(join(runRoot, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  await persistManifest();
  await writeFile(join(runRoot, "assertions.json"), "[]\n");
  await event({ kind: "run_started" });
  const processes = [];
  let cleanupResult;

  function startProcess(command, args, { cwd: childCwd = cwd, env = {}, timeoutMs = 900_000, role = basename(command) } = {}) {
    const child = spawn(command, args, {
      cwd: childCwd, env: safeEnvironment(env), shell: false, windowsHide: true,
      detached: process.platform !== "win32", stdio: ["ignore", "pipe", "pipe"],
    });
    const record = { pid: child.pid ?? null, role, command, args: args.map(redacted), cwd: childCwd, started_at: new Date().toISOString(), exit_code: null, signal: null, closed: false, absent: false, cancelled_tree_pids: [] };
    const index = processes.length;
    let stdout = "", stderr = "", error, timedOut = false, cancellation;
    const limit = 1024 * 1024;
    child.stdout.on("data", (chunk) => { stdout = (stdout + chunk.toString()).slice(-limit); });
    child.stderr.on("data", (chunk) => { stderr = (stderr + chunk.toString()).slice(-limit); });
    child.once("error", (cause) => { error = cause; });
    const exited = new Promise((resolveExited) => child.once("close", (code, signal) => {
      record.exit_code = code;
      record.signal = signal;
      record.closed = true;
      record.exited_at = new Date().toISOString();
      clearTimeout(timer);
      resolveExited();
    }));
    const stop = () => cancellation ??= (async () => {
      if (record.closed && record.absent) return;
      if (!record.closed && record.pid) record.cancelled_tree_pids = await stopTree(record.pid);
      await bounded(exited, 2000, "JOIN_TIMEOUT");
      record.absent = !record.pid || !alive(record.pid);
      record.absence_observed_at = new Date().toISOString();
      if (!record.absent) throw failure("CLEANUP_FAILED", `owned PID ${record.pid} remains alive`);
    })();
    const timer = setTimeout(() => {
      timedOut = true;
      stop().catch((cause) => { error = cause; });
    }, timeoutMs);
    const completed = (async () => {
      await bounded(exited, timeoutMs + 8000, "PROCESS_JOIN_TIMEOUT");
      if (cancellation) await cancellation;
      record.absent = !record.pid || !alive(record.pid);
      record.absence_observed_at = new Date().toISOString();
      const log = `stdout:\n${redacted(stdout)}\nstderr:\n${redacted(stderr)}`;
      record.log_path = join("logs", `${index}-${role.replace(/[^\w-]/g, "_")}.txt`);
      await writeFile(join(runRoot, record.log_path), log);
      await event({ kind: "process_exited", ...record });
      return { stdout: redacted(stdout), stderr: redacted(stderr), pid: record.pid };
    })();
    // Services finish during cleanup; attach rejection observation immediately.
    completed.catch(() => {});
    const handle = { record, completed, stop, getError: () => error, timedOut: () => timedOut };
    processes.push(handle);
    void event({ kind: "process_started", pid: record.pid, role, command });
    return handle;
  }

  async function runProcess(command, args, options) {
    const handle = startProcess(command, args, options);
    const result = await handle.completed;
    if (handle.getError()) throw handle.getError();
    if (handle.timedOut()) throw failure("PROCESS_TIMEOUT", `${basename(command)} exceeded its deadline`);
    if (handle.record.exit_code !== 0) throw failure("PROCESS_FAILED", `${basename(command)} exited ${handle.record.exit_code ?? handle.record.signal}: ${result.stderr.slice(-1000)}`);
    return result;
  }

  async function cleanup() {
    if (cleanupResult) return cleanupResult;
    const errors = [];
    const stopped = await Promise.allSettled(processes.map(async (handle) => {
      await handle.stop();
      await handle.completed;
    }));
    for (const result of stopped) if (result.status === "rejected") errors.push(result.reason.message);
    try { await rm(fixtureRoot, { recursive: true, force: true }); }
    catch (error) { errors.push(error.message); }
    let fixtureAbsent = false;
    try { await stat(fixtureRoot); }
    catch (error) { if (error.code === "ENOENT") fixtureAbsent = true; else errors.push(error.message); }
    const remaining = processes.filter(({ record }) => !record.closed || !record.absent).length + Number(!fixtureAbsent);
    cleanupResult = {
      owned_ids: [runId], owned_resources: processes.map(({ record }) => record),
      exit_join_result: errors.length || remaining ? "failed" : "joined",
      fixture_absent: fixtureAbsent, errors, preserved_artifact_root: root,
      preserved_ledger_path: resolve(budgetLedger), remaining_resource_count: remaining,
    };
    await writeFile(join(runRoot, "cleanup.json"), `${JSON.stringify(cleanupResult, null, 2)}\n`);
    await event({ kind: "cleanup_completed", remaining_resource_count: remaining, outcome: cleanupResult.exit_join_result });
    if (errors.length || remaining) throw failure("CLEANUP_FAILED", errors.join("; ") || `${remaining} resources remain`);
    return cleanupResult;
  }
  return { root, runRoot, fixtureRoot, manifest, event, persistManifest, spawn: runProcess, startProcess, cleanup };
}
