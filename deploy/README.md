# Deploying the web demo

`prod` moves → GitHub Actions builds the image and pushes it to GHCR → the
workflow makes one signed HTTPS request to the server → the server pulls the
image and swaps the container.

Nothing connects *into* the server over SSH, which is the point: the server is
too small to compile Rust, and an inbound SSH session from a fresh GitHub runner
IP sets off the cloud provider's unusual-login alarm on every deploy. Here the
runner does the building, and the only thing it asks the server for is a pull.

The files in this directory live on the server. They are checked in so they are
reviewable and versioned, but they are **copied there by hand** — the deploy
flow does not update them. Changing `deploy.sh` means copying it up and
re-running `bootstrap.sh`, which is the trade for having no inbound shell.

## How it fits the server

Worth knowing before reading the scripts, because it explains three things that
would otherwise look arbitrary:

- **nginx runs in a container**, on the `app_network` bridge, with its config
  bind-mounted from `/home/ubuntu/docker/nginx/conf.d`. It reaches the app by
  container name, so the app **publishes no host port at all** — `deploy.sh`
  starts it with `--network app_network` and no `-p`.
- **The receiver binds the bridge gateway** (`172.19.0.1:9001`), not loopback.
  A containerised nginx cannot reach the host's `127.0.0.1`. `bootstrap.sh`
  looks the gateway up and writes it to `/etc/lexiroot/webhook.env`; if the
  network is ever recreated on a different subnet, re-running bootstrap is the
  fix. Port 9001 rather than webhook's usual 9000, which clash already holds.
- **`deploy.sh` reloads nginx after the swap.** nginx resolves the names in a
  static `upstream` block once, at load time, and caches the result for the life
  of the process. A recreated container usually comes back on a different
  address, so without the reload the site 502s until something else reloads it.

## One-time setup

### 1. Repository secrets

Under *Settings → Secrets and variables → Actions*:

| Secret | Value |
| --- | --- |
| `DEPLOY_WEBHOOK_URL` | `https://lexiroot.viii.me/hooks/deploy` |
| `DEPLOY_WEBHOOK_SECRET` | `openssl rand -hex 32` |

Optional variable `DEPLOY_IMAGE` overrides the image name, which otherwise
defaults to `ghcr.io/<owner>/<repo>`.

### 2. Server

Copy this directory up and run the bootstrap, passing the same secret:

```sh
scp deploy/{deploy.sh,hooks.json,lexiroot-webhook.service,bootstrap.sh} \
    ubuntu@lexiroot.viii.me:/tmp/lexiroot-deploy/
ssh ubuntu@lexiroot.viii.me \
    "sudo DEPLOY_WEBHOOK_SECRET=<the same hex string> /tmp/lexiroot-deploy/bootstrap.sh"
```

It creates the `deploy` user, installs `deploy.sh` to `/opt/lexiroot`, writes
`/etc/lexiroot/hooks.json` with the secret substituted in (mode `640
root:deploy`, since it holds the value in cleartext) and `webhook.env` with the
bridge gateway, installs and starts the receiver, then checks that an *unsigned*
request is refused before it finishes. Idempotent; re-run it to pick up an
edited `deploy.sh`.

### 3. nginx

In `/home/ubuntu/docker/nginx/conf.d/lexiroot.viii.me.conf`, at the top of the
file (conf.d is included inside `http`, so this is the right context):

```nginx
limit_req_zone $binary_remote_addr zone=deploy:1m rate=6r/m;
```

and in the `:443` server block, *before* `location /`:

```nginx
location /hooks/ {
    limit_req zone=deploy burst=5 nodelay;
    proxy_pass http://172.19.0.1:9001/hooks/;

    # The receiver blocks until deploy.sh finishes so a failed deploy answers
    # 5xx and turns the GitHub run red. Give it room to pull and health-check.
    proxy_read_timeout 300s;
}
```

Then `docker exec nginx nginx -t && docker exec nginx nginx -s reload`.

### 4. First deploy, and the GHCR visibility trap

The first push to `prod` creates the GHCR package **private**, because that is
GitHub's default. The build and push will succeed, the webhook will fire, and
the server's `docker pull` will fail with `denied` — so expect the first run to
go red at the last step.

Fix it once and it stays fixed: open the package under the repository's
*Packages* tab, set visibility to public, then re-run the workflow from the
Actions tab. Public is what lets the server pull with **no registry credentials
stored on it at all**.

If you would rather keep it private, log in on the server as the deploy user
with a read-only PAT — `DOCKER_CONFIG` already points at `/opt/lexiroot/.docker`
— and that PAT becomes a credential to rotate.

## Operating it

**Deploy by hand** (e.g. after editing `deploy.sh`):

```sh
sudo -u deploy /opt/lexiroot/deploy.sh
```

Safe to run any time — if the live container is already on the current `prod`
image, it exits without touching anything.

**Roll back.** A deploy that fails its health check rolls itself back. For one
that passed but was wrong anyway, the outgoing image is still tagged locally:

```sh
sudo -u deploy docker rm -f lexiroot
sudo -u deploy docker run -d --name lexiroot --restart unless-stopped \
    --network app_network lexiroot:previous
sudo docker exec nginx nginx -s reload
```

Only one generation is kept on disk. To go further back, pull a specific build
by commit — every build is also pushed as `sha-<full commit sha>`:

```sh
sudo -u deploy docker pull ghcr.io/tcyeee/lexiroot:sha-<sha>
```

**Watch the receiver:** `journalctl -u lexiroot-webhook -f`.

**Check the endpoint is still refusing unsigned requests:**

```sh
curl -si -X POST -d '{"revision":"probe"}' https://lexiroot.viii.me/hooks/deploy | head -1
# expect: HTTP/2 403
```

## What a leaked webhook secret gets you

A redeploy of the image already tagged `prod`, and nothing else. The receiver
runs one fixed script; `deploy.sh` chooses its own image and ignores the request
body apart from echoing the commit into the log. There is no path from the
request to an arbitrary image or an arbitrary command.

Note that webhook answers `200` for an unsatisfied trigger rule unless the hook
sets `trigger-rule-mismatch-http-response-code`, which `hooks.json` does. That
is why the check above expects a `403` specifically: a bare `200` cannot be told
apart from "no rule was applied and the script just ran".

The weaker link is the `docker` group: membership is effectively root on this
host, so anyone who can run `deploy.sh` as `deploy` can escalate. On a
single-purpose box that is the normal trade. Tightening it means dropping the
group and giving the receiver a `sudoers` entry for exactly
`/opt/lexiroot/deploy.sh` instead.
