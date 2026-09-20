#!/usr/bin/env bash
set -euo pipefail

# Generic local-dev convenience wrapper: builds and starts this template's
# container image, then waits for /health. Ported from file-combiner's
# scripts/start-local-container.sh, generalized to drop file-combiner's
# private-dependency (agent-builder SSH key) and GitHub CLI requirements,
# which this template does not have.
IMAGE="template:local"
CONTAINER_NAME="template-local"
PORT="${PORT:-8081}"
BASE_URL="http://127.0.0.1:${PORT}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! docker info >/dev/null 2>&1; then
  echo "Starting Docker Desktop..."
  powershell.exe -NoProfile -Command 'Start-Process -FilePath "$env:ProgramFiles\Docker\Docker\Docker Desktop.exe"'
  for _ in $(seq 1 120); do
    if docker info >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done
  docker info >/dev/null 2>&1 || {
    echo "Docker Desktop did not become ready within two minutes." >&2
    exit 1
  }
fi

cd "$REPO_ROOT"
docker build -t "$IMAGE" .
docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
docker run -d --rm --name "$CONTAINER_NAME" -p "${PORT}:8080" "$IMAGE" >/dev/null

for _ in $(seq 1 30); do
  if curl -sf "${BASE_URL}/health" >/dev/null; then
    echo "template server is available at ${BASE_URL}"
    exit 0
  fi
  sleep 1
done

echo "${CONTAINER_NAME} did not become healthy" >&2
exit 1
