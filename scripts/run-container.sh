#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${IMAGE:-template:local}"
CONTAINER_NAME="${CONTAINER_NAME:-template-local}"
PORT="${PORT:-8080}"

cd "$REPO_ROOT"

echo "Building ${IMAGE}..."
docker build -t "$IMAGE" .

echo "Starting ${CONTAINER_NAME} at http://127.0.0.1:${PORT}"

docker rm -f "$CONTAINER_NAME" > /dev/null 2>&1 || true

exec docker run --rm \
  --name "$CONTAINER_NAME" \
  -p "${PORT}:8080" \
  "$IMAGE"