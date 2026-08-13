#!/usr/bin/env bash
#
# Swaps the running container onto the image GitHub Actions just published.
#
# This script lives on the server (see README.md in this directory) and is run
# by the webhook receiver after it verifies the request signature. It is the
# only thing that touches the running site, so it is written to be safe when
# run twice, and to leave something serving even when the new image is broken.
#
# The argument is the commit the image was built from. It is used for logging
# only — the image that gets deployed is always the `prod` tag of the image
# named below, never anything the caller supplies. That way a leaked webhook
# secret buys an attacker a redeploy of the current release and nothing more.
#
# Usage: deploy.sh [revision]

set -euo pipefail

IMAGE="${DEPLOY_IMAGE:-ghcr.io/tcyeee/lexiroot}"
TAG="${DEPLOY_TAG:-prod}"
CONTAINER="${DEPLOY_CONTAINER:-lexiroot}"
# The container publishes no host port. nginx runs in a container of its own on
# this network and reaches the app by DNS name, so the app never has to be
# exposed on the host at all.
NETWORK="${DEPLOY_NETWORK:-app_network}"
APP_PORT="${DEPLOY_APP_PORT:-8080}"
PROXY_CONTAINER="${DEPLOY_PROXY_CONTAINER:-nginx}"
REVISION="${1:-unknown}"

# The deploy user has no home directory, so the docker CLI warns on every run
# about the config it cannot read. Point it somewhere the user owns — which is
# also where a registry credential would go if the package ever went private.
export DOCKER_CONFIG="${DOCKER_CONFIG:-/opt/lexiroot/.docker}"

# Serialise deploys. The workflow's concurrency group already prevents two
# GitHub runs from overlapping, but a manual run on the server should not be
# able to collide with an automatic one.
exec 9>/var/lock/lexiroot-deploy.lock
if ! flock -n 9; then
  echo "Another deploy is already running; refusing to start a second." >&2
  exit 1
fi

echo "Deploying revision ${REVISION} from ${IMAGE}:${TAG}"

# The image ID the live container was created from. This survives the `prod`
# tag moving to a new image, so it stays a valid rollback target below. Empty
# on the very first deploy, when nothing is running yet.
previous_image="$(docker inspect --format '{{.Image}}' "$CONTAINER" 2>/dev/null || true)"

docker pull "$IMAGE:$TAG"
new_image="$(docker inspect --format '{{.Id}}' "$IMAGE:$TAG")"

if [ "$new_image" = "$previous_image" ]; then
  echo "Already running ${new_image}; nothing to do."
  exit 0
fi

# Give the outgoing image a name of its own before the swap. Without it the
# image goes dangling the moment `prod` moves and the prune at the end deletes
# it, leaving no way to roll back by hand after a deploy that passed its health
# check but turned out to be wrong anyway.
if [ -n "$previous_image" ]; then
  docker tag "$previous_image" "$CONTAINER:previous"
fi

start_container() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker run -d \
    --name "$CONTAINER" \
    --restart unless-stopped \
    --network "$NETWORK" \
    "$1" >/dev/null
}

container_ip() {
  docker inspect \
    --format "{{index .NetworkSettings.Networks \"$NETWORK\" \"IPAddress\"}}" \
    "$CONTAINER" 2>/dev/null
}

# Checked against the container's own address rather than through nginx, so a
# failure here means the app is broken and not that the proxy is stale.
healthy() {
  for _ in $(seq 1 20); do
    ip="$(container_ip || true)"
    if [ -n "$ip" ] && curl -fsS --max-time 3 "http://$ip:$APP_PORT/" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  return 1
}

# nginx resolves the names in a static `upstream` block once, at load time, and
# caches the result for the life of the process. A recreated container usually
# comes back on a different address, so without this reload the site 502s until
# something else happens to reload nginx.
reload_proxy() {
  if ! docker inspect "$PROXY_CONTAINER" >/dev/null 2>&1; then
    echo "Proxy container '$PROXY_CONTAINER' not found; skipping reload." >&2
    return 0
  fi
  if docker exec "$PROXY_CONTAINER" nginx -s reload >/dev/null 2>&1; then
    echo "Reloaded ${PROXY_CONTAINER}."
  else
    echo "Failed to reload ${PROXY_CONTAINER}; it may still be serving the old address." >&2
    return 1
  fi
}

start_container "$new_image"

if healthy; then
  echo "Health check passed (container at $(container_ip):${APP_PORT})."
else
  echo "Health check failed; container logs follow." >&2
  docker logs --tail 50 "$CONTAINER" >&2 || true
  if [ -n "$previous_image" ]; then
    echo "Rolling back to ${previous_image}." >&2
    start_container "$previous_image"
    healthy || echo "The rolled-back container is not healthy either." >&2
    reload_proxy || true
  else
    echo "No previous image to roll back to." >&2
  fi
  exit 1
fi

reload_proxy

# Only reaches images that are neither running nor tagged `previous`, i.e. two
# generations old. Worth doing on a small disk: each build is ~100MB.
docker image prune -f >/dev/null

echo "Now serving ${new_image} (revision ${REVISION})."
