#!/usr/bin/env bash
#
# One-time server setup for the deploy flow. Run as root, from this directory:
#
#   sudo DEPLOY_WEBHOOK_SECRET=<hex> ./bootstrap.sh
#
# Idempotent — safe to re-run after changing deploy.sh or the unit file, which
# is the normal way to update them, since nothing pushes them here automatically.
#
# It does everything except the nginx change, which it prints at the end instead
# of applying: editing a live server block that terminates TLS for the site is
# not something a script should do behind your back.

set -euo pipefail

SECRET="${DEPLOY_WEBHOOK_SECRET:-}"
NETWORK="${DEPLOY_NETWORK:-app_network}"
WEBHOOK_PORT="${DEPLOY_WEBHOOK_PORT:-9001}"
APP_DIR=/opt/lexiroot
CONF_DIR=/etc/lexiroot
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$(id -u)" -ne 0 ]; then
  echo "Run me as root (sudo)." >&2
  exit 1
fi

if [ -z "$SECRET" ]; then
  echo "Set DEPLOY_WEBHOOK_SECRET to the same value as the repository secret." >&2
  echo "Generate one with: openssl rand -hex 32" >&2
  exit 1
fi

for f in deploy.sh hooks.json lexiroot-webhook.service; do
  [ -f "$SRC_DIR/$f" ] || { echo "Missing $SRC_DIR/$f" >&2; exit 1; }
done

echo "==> Checking prerequisites"
command -v docker >/dev/null || { echo "docker is not installed." >&2; exit 1; }
# deploy.sh health-checks the new container through curl on the host.
command -v curl >/dev/null || { echo "curl is not installed." >&2; exit 1; }
docker network inspect "$NETWORK" >/dev/null 2>&1 \
  || { echo "Docker network '$NETWORK' does not exist." >&2; exit 1; }

# nginx runs in a container and cannot reach the host's loopback, so the
# receiver listens on the host's address on the app network's bridge instead.
GATEWAY="$(docker network inspect "$NETWORK" \
  --format '{{range .IPAM.Config}}{{.Gateway}}{{end}}')"
if [ -z "$GATEWAY" ]; then
  echo "Could not determine the gateway address of '$NETWORK'." >&2
  exit 1
fi
echo "    receiver will bind ${GATEWAY}:${WEBHOOK_PORT} (bridge for ${NETWORK})"

if ss -lnt "sport = :${WEBHOOK_PORT}" | grep -q LISTEN; then
  if ! systemctl is-active --quiet lexiroot-webhook; then
    echo "Port ${WEBHOOK_PORT} is already in use by something else." >&2
    exit 1
  fi
fi

if ! command -v webhook >/dev/null; then
  echo "==> Installing webhook"
  apt-get update -qq
  apt-get install -y -qq webhook
  # Debian's package ships its own unit; we run our own with a different name
  # and config path, so make sure the packaged one is not also listening.
  systemctl disable --now webhook >/dev/null 2>&1 || true
fi

echo "==> Creating the deploy user"
if ! id -u deploy >/dev/null 2>&1; then
  adduser --system --group --no-create-home deploy
fi
# Membership in `docker` is effectively root on this host. See README.md.
usermod -aG docker deploy

echo "==> Installing files"
install -d -o deploy -g deploy -m 755 "$APP_DIR"
install -d -o root   -g deploy -m 750 "$CONF_DIR"
# DOCKER_CONFIG points here; see the note in deploy.sh.
install -d -o deploy -g deploy -m 700 "$APP_DIR/.docker"
install -o deploy -g deploy -m 750 "$SRC_DIR/deploy.sh" "$APP_DIR/deploy.sh"

printf 'WEBHOOK_IP=%s\nWEBHOOK_PORT=%s\n' "$GATEWAY" "$WEBHOOK_PORT" \
  > "$CONF_DIR/webhook.env"
chown root:deploy "$CONF_DIR/webhook.env"
chmod 640 "$CONF_DIR/webhook.env"

# The secret goes in at install time so it never has to be committed. Written
# via a temp file in the destination directory so the real path is never
# briefly world-readable.
tmp="$(mktemp "$CONF_DIR/hooks.json.XXXXXX")"
chmod 640 "$tmp"
chown root:deploy "$tmp"
sed "s|REPLACE_WITH_THE_SHARED_SECRET|$SECRET|" "$SRC_DIR/hooks.json" > "$tmp"
mv "$tmp" "$CONF_DIR/hooks.json"

if grep -q REPLACE_WITH_THE_SHARED_SECRET "$CONF_DIR/hooks.json"; then
  echo "Secret substitution failed; the placeholder is still in hooks.json." >&2
  exit 1
fi

install -o root -g root -m 644 \
  "$SRC_DIR/lexiroot-webhook.service" /etc/systemd/system/lexiroot-webhook.service

echo "==> Starting the receiver"
systemctl daemon-reload
systemctl enable lexiroot-webhook >/dev/null
systemctl restart lexiroot-webhook

sleep 2
if ! systemctl is-active --quiet lexiroot-webhook; then
  echo "The receiver failed to start. Recent logs:" >&2
  journalctl -u lexiroot-webhook -n 30 --no-pager >&2
  exit 1
fi

# An unsigned request must be refused.
#
# webhook answers 200 for an unsatisfied trigger rule unless the hook sets
# trigger-rule-mismatch-http-response-code, so a bare 200 here is ambiguous: it
# could mean the rule rejected the request, or that no rule was applied at all
# and deploy.sh just ran. Insisting on the 403 that hooks.json asks for makes
# the two cases distinguishable.
echo "==> Verifying that unsigned requests are rejected"
code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 \
  -X POST -H 'Content-Type: application/json' \
  --data '{"revision":"unsigned-probe"}' \
  "http://${GATEWAY}:${WEBHOOK_PORT}/hooks/deploy" || true)"
if [ "$code" != "403" ]; then
  echo "Expected HTTP 403 for an unsigned request but got ${code}." >&2
  if [ "$code" = "200" ]; then
    echo "A 200 means either the signature check is not being enforced, or" >&2
    echo "trigger-rule-mismatch-http-response-code is missing from hooks.json." >&2
  fi
  echo "Stopping the receiver until this is resolved." >&2
  systemctl stop lexiroot-webhook
  exit 1
fi
echo "    rejected with HTTP 403, as expected"

cat <<NGINX

==> Remaining manual step: nginx

Add to the :443 server block of lexiroot.viii.me.conf:

    location /hooks/ {
        limit_req zone=deploy burst=5 nodelay;
        proxy_pass http://${GATEWAY}:${WEBHOOK_PORT}/hooks/;

        # The receiver blocks until deploy.sh finishes so a failed deploy
        # answers 5xx and turns the GitHub run red. Give it room.
        proxy_read_timeout 300s;
    }

and to the http block (nginx.conf):

    limit_req_zone \$binary_remote_addr zone=deploy:1m rate=6r/m;

Then test and reload the proxy container:

    docker exec nginx nginx -t && docker exec nginx nginx -s reload

NGINX

echo "==> Done. Follow the receiver with: journalctl -u lexiroot-webhook -f"
