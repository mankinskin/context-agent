#!/usr/bin/env bash
set -euo pipefail

# Generic browser smoke for any app built from this template: builds the
# frontend, starts the backend, and runs the Playwright spec in
# frontend/e2e against it. No credentials or fake-backend flags are needed
# because this template's /api/echo has no auth of its own; a tool built
# from this template that adds auth should extend this script accordingly.
PORT="${PORT:-8080}"
BASE_URL="http://127.0.0.1:${PORT}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$REPO_ROOT"

BACKEND_PID=""
cleanup() {
  if [[ -n "$BACKEND_PID" ]]; then
    kill "$BACKEND_PID" 2>/dev/null || true
    wait "$BACKEND_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

(cd frontend && trunk build --release)

PORT="$PORT" STATIC_DIR="frontend/dist" cargo run -p backend &
BACKEND_PID=$!

READY=""
for _ in $(seq 1 30); do
  if curl -sf "${BASE_URL}/health" > /dev/null; then
    READY=1
    break
  fi
  sleep 1
done
[[ -n "$READY" ]] || { echo "backend did not become healthy in time" >&2; exit 1; }

npm ci --prefix frontend/e2e
npx --prefix frontend/e2e playwright install chromium
BASE_URL="$BASE_URL" npm test --prefix frontend/e2e
