#!/usr/bin/env bash
# Builds the image here and swaps it in on the server:
#   1. build pixiu:alpine locally
#   2. on the server, rename the running image to pixiu:alpine-prev
#   3. upload the new pixiu:alpine
#   4. recreate the container from it and wait until it is healthy
#   5. delete the old image
# If the new container never gets healthy, the old image is kept (step 5 is
# skipped) so you can go back to it.
#
# Settings (environment): HOST (default vps), COMPOSE_DIR (default
# /root/docker-compose/pixiu).

set -euo pipefail

HOST=${HOST:-vps}
COMPOSE_DIR=${COMPOSE_DIR:-/root/docker-compose/pixiu}
IMAGE=pixiu:alpine
PREV=pixiu:alpine-prev

cd "$(dirname "$0")"

echo "==> Building $IMAGE"
docker build -t "$IMAGE" .

# The server's login shell is fish, so remote steps run through sh.
remote() {
    ssh -o BatchMode=yes "$HOST" sh -s -- "$@"
}

echo "==> Renaming the server's $IMAGE to $PREV"
remote "$IMAGE" "$PREV" <<'EOF'
set -eu
# A leftover from an earlier deploy would otherwise turn into an untagged image.
docker rmi "$2" >/dev/null 2>&1 || true
if docker image inspect "$1" >/dev/null 2>&1; then
    docker tag "$1" "$2"
    docker rmi "$1" >/dev/null
else
    echo "(no $1 on the server yet)"
fi
EOF

echo "==> Uploading $IMAGE"
docker save "$IMAGE" | gzip -1 | ssh -o BatchMode=yes "$HOST" docker load

echo "==> Recreating the container"
remote "$COMPOSE_DIR" "$PREV" <<'EOF'
set -eu
cd "$1"
docker compose up -d --force-recreate
container=$(docker compose ps -q | head -n 1)
status=starting
for _ in $(seq 1 45); do
    status=$(docker inspect -f '{{.State.Health.Status}}' "$container")
    [ "$status" = healthy ] && break
    sleep 2
done
if [ "$status" != healthy ]; then
    echo "The new container is $status after 90 s; keeping $2." >&2
    docker compose logs --tail 30 >&2
    exit 1
fi
echo "healthy"

echo "==> Deleting the old image"
docker rmi "$2" >/dev/null 2>&1 || echo "(no old image to delete)"
EOF

echo "==> Deployed"
