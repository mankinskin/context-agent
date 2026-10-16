import { failure } from "./e2e-lifecycle.mjs";

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

export function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" && argv.length === 1) return { help: true };
    if (!["--lane", "--scenario", "--media", "--artifact-root", "--budget-ledger", "--verify-artifacts"].includes(arg)) {
      throw failure("INVALID_ARGUMENT", `unknown argument: ${arg}`);
    }
    const value = argv[++index];
    if (!value || value.startsWith("--")) throw failure("INVALID_ARGUMENT", `invalid value for ${arg}`);
    const key = arg.slice(2).replaceAll("-", "_");
    if (key in options) throw failure("INVALID_ARGUMENT", `duplicate argument: ${arg}`);
    options[key] = value;
  }
  if (options.verify_artifacts) {
    if (Object.keys(options).length !== 1) throw failure("INVALID_ARGUMENT", "--verify-artifacts is exclusive and read-only");
    return options;
  }
  options.media ??= "desktop";
  for (const name of ["lane", "scenario", "artifact_root", "budget_ledger"]) {
    if (!options[name]) throw failure("INVALID_ARGUMENT", `invalid: --${name.replaceAll("_", "-")} is required`);
  }
  if (!["offline", "container", "live"].includes(options.lane)) throw failure("INVALID_ARGUMENT", `invalid lane: ${options.lane}`);
  if (!["baseline", "bridge", "sessions", "editor-git", "spine", "choice", "choice-mcp", "all"].includes(options.scenario)) throw failure("UNSUPPORTED_SCENARIO", `unsupported scenario: ${options.scenario}`);
  mediaProjects(options.media);
  if (options.lane === "live" && options.media === "all") throw failure("INVALID_MEDIA", "live requires one origin media project");
  if (options.lane === "live" && !["spine", "choice-mcp", "all"].includes(options.scenario)) throw failure("UNSUPPORTED_SCENARIO", "live requires scenario: spine, choice-mcp or all");
  if (options.lane === "container" && options.scenario !== "baseline") throw failure("UNSUPPORTED_SCENARIO", "container requires baseline");
  if (options.lane === "offline" && ["spine", "choice-mcp"].includes(options.scenario)) throw failure("UNSUPPORTED_SCENARIO", "spine and choice-mcp are live-only scenarios");
  return options;
}
