# MVRE Polar Drift Hub

A minimal CLI for provisioning and operating a MoSAiC Virtual Research Environment (JupyterHub + Traefik) via Docker Compose.

Arctic mission focus, minimal surface area, fast setup.

## Install
```bash
cargo install --path .
```

## Features
- Interactive or non-interactive deployment
- Config persistence (remembers last deployment path and domain)
- Safer cleanup with explicit confirmation flag
- Systemd service install (optional, root-only)
- Clear logging with `-v`/`-vv`

## Usage

### Deploy
Creates a deployment directory, writes configuration, and prepares Docker Compose + JupyterHub.
```bash
mvre-hub deploy
```

Non-interactive deploy:
```bash
mvre-hub deploy --domain hub.example.org --acme-email admin@example.org \
  --client-id <id> --client-secret <secret> \
  --dataset-path /data/mosaic \
  --oauth-authorize-url https://issuer/authorize \
  --oauth-token-url https://issuer/token \
  --oauth-userdata-url https://issuer/userinfo \
  --install-notebooks
```

Testing without a valid dataset path:
```bash
mvre-hub deploy --dataset-path ./data --allow-missing-dataset
```

Production profile (Postgres + culling + limits):
```bash
mvre-hub deploy --production
```

### Start/Stop
`start` builds images (if needed) and launches JupyterHub + Traefik.  
`stop` cleanly shuts down the services but keeps data.
```bash
mvre-hub start
mvre-hub stop
```

Override deployment directory:
```bash
mvre-hub --deploy-dir /path/to/deploy start
```

### Preflight
Validates local readiness (docker, ports, dataset path, DNS) before deploy/start.
```bash
mvre-hub preflight
```

Fail on warnings:
```bash
mvre-hub preflight --strict
```

### Cleanup
Stops services, removes containers/images/volumes, and deletes the deployment directory.
```bash
mvre-hub clean --full-ice
```

## Configuration
Default config path:
- `~/.config/mvre-hub/config.json`

Override:
```bash
mvre-hub --config /path/to/config.json start
```

## Notes
- Requires `docker-compose` binary available on `PATH`.
- Systemd integration writes `/etc/systemd/system/mvre-hub.service`.
- `mvre-hub start` builds the hub and user images before starting services.
