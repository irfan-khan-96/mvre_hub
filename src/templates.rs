pub fn docker_compose(domain: &str, acme_email: &str, production: bool) -> String {
    let mut base = format!(
        r#"services:
  jupyterhub:
    build: ./hub
    env_file: .env
    volumes:
      - ./hub/jupyterhub_config.py:/etc/jupyterhub/jupyterhub_config.py:ro
      - ./jupyterhub_data:/srv/jupyterhub
      - /var/run/docker.sock:/var/run/docker.sock
    {depends_on}
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.jupyterhub.rule=Host(`{domain}`)"
      - "traefik.http.routers.jupyterhub.entrypoints=websecure"
      - "traefik.http.routers.jupyterhub.tls=true"
      - "traefik.http.routers.jupyterhub.tls.certresolver=letsencrypt"
    command: ["jupyterhub", "-f", "/etc/jupyterhub/jupyterhub_config.py"]

  user-image:
    build: ./user
    image: ${{USER_IMAGE}}
    command: ["true"]

  traefik:
    image: traefik:v2.9
    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.tlschallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.email={acme_email}"
      - "--certificatesresolvers.letsencrypt.acme.storage=/certs/acme.json"
    ports:
      - "8080:80"
      - "8443:443"
    volumes:
      - ./traefik:/certs
      - /var/run/docker.sock:/var/run/docker.sock:ro
"#,
        domain = domain,
        depends_on = if production {
            "depends_on:\n      - postgres"
        } else {
            ""
        }
    );

    if production {
        base.push_str(
            r#"

  postgres:
    image: postgres:15
    environment:
      POSTGRES_USER: ${DB_USER}
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: ${DB_NAME}
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
"#,
        );
    }

    base
}

pub struct EnvValues<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub domain: &'a str,
    pub user_image: &'a str,
    pub dataset_host: &'a str,
    pub dataset_mount: &'a str,
    pub allow_missing_dataset: bool,
    pub shared_host: Option<&'a str>,
    pub shared_mount: &'a str,
    pub admin_users: Option<&'a str>,
    pub oauth_authorize_url: Option<&'a str>,
    pub oauth_token_url: Option<&'a str>,
    pub oauth_userdata_url: Option<&'a str>,
    pub oauth_username_key: &'a str,
    pub production: bool,
    pub db_user: &'a str,
    pub db_name: &'a str,
    pub db_password: &'a str,
    pub db_host: &'a str,
    pub db_port: u16,
    pub cpu_limit: Option<&'a str>,
    pub mem_limit: Option<&'a str>,
    pub cull_timeout: Option<u64>,
    pub cull_every: Option<u64>,
}

pub fn env_file(values: &EnvValues) -> String {
    format!(
        "HUB_DOMAIN={}\nOAUTH_CLIENT_ID={}\nOAUTH_CLIENT_SECRET={}\nUSER_IMAGE={}\nDATASET_HOST_PATH={}\nDATASET_MOUNT_PATH={}\nALLOW_MISSING_DATASET={}\nSHARED_HOST_PATH={}\nSHARED_MOUNT_PATH={}\nADMIN_USERS={}\nOAUTH_AUTHORIZE_URL={}\nOAUTH_TOKEN_URL={}\nOAUTH_USERDATA_URL={}\nOAUTH_USERNAME_KEY={}\nENABLE_POSTGRES={}\nDB_USER={}\nDB_PASSWORD={}\nDB_NAME={}\nDB_HOST={}\nDB_PORT={}\nJUPYTERHUB_DB_URL={}\nCPU_LIMIT={}\nMEM_LIMIT={}\nCULL_TIMEOUT={}\nCULL_EVERY={}\nALLOW_DUMMY_AUTH=false\n",
        values.domain,
        values.client_id,
        values.client_secret,
        values.user_image,
        values.dataset_host,
        values.dataset_mount,
        values.allow_missing_dataset,
        values.shared_host.unwrap_or(""),
        values.shared_mount,
        values.admin_users.unwrap_or(""),
        values.oauth_authorize_url.unwrap_or(""),
        values.oauth_token_url.unwrap_or(""),
        values.oauth_userdata_url.unwrap_or(""),
        values.oauth_username_key,
        values.production,
        values.db_user,
        values.db_password,
        values.db_name,
        values.db_host,
        values.db_port,
        if values.production {
            format!(
                "postgresql://{}:{}@{}:{}/{}",
                values.db_user,
                values.db_password,
                values.db_host,
                values.db_port,
                values.db_name
            )
        } else {
            "".to_string()
        },
        values.cpu_limit.unwrap_or(""),
        values.mem_limit.unwrap_or(""),
        values
            .cull_timeout
            .map(|v| v.to_string())
            .unwrap_or_else(String::new),
        values
            .cull_every
            .map(|v| v.to_string())
            .unwrap_or_else(String::new),
    )
}

pub fn jupyterhub_config() -> String {
    r#"
import os

from dockerspawner import DockerSpawner
from oauthenticator.generic import GenericOAuthenticator

c = get_config()

c.JupyterHub.spawner_class = DockerSpawner
c.JupyterHub.hub_ip = "0.0.0.0"
c.JupyterHub.hub_connect_ip = "jupyterhub"
c.JupyterHub.bind_url = "http://:8000"

db_url = os.environ.get("JUPYTERHUB_DB_URL")
if db_url:
    c.JupyterHub.db_url = db_url

c.DockerSpawner.image = os.environ.get("USER_IMAGE", "mvre-user:latest")
c.DockerSpawner.network_name = os.environ.get("DOCKER_NETWORK_NAME", "mvre-hub_default")
c.DockerSpawner.remove = True
c.DockerSpawner.use_internal_ip = True
c.Spawner.notebook_dir = "/home/jovyan/work"

volumes = {"jupyterhub-user-{username}": "/home/jovyan/work"}

dataset_host = os.environ.get("DATASET_HOST_PATH")
dataset_mount = os.environ.get("DATASET_MOUNT_PATH", "/data/mosaic")
if dataset_host:
    volumes[dataset_host] = {"bind": dataset_mount, "mode": "ro"}

shared_host = os.environ.get("SHARED_HOST_PATH")
shared_mount = os.environ.get("SHARED_MOUNT_PATH", "/home/jovyan/shared")
if shared_host:
    volumes[shared_host] = {"bind": shared_mount, "mode": "ro"}

c.DockerSpawner.volumes = volumes

env = {"MOSAIC_DATA": dataset_mount}
if shared_host:
    env["MOSAIC_SHARED"] = shared_mount
c.Spawner.environment = env

cpu_limit = os.environ.get("CPU_LIMIT")
mem_limit = os.environ.get("MEM_LIMIT")
if cpu_limit:
    c.DockerSpawner.cpu_limit = float(cpu_limit)
if mem_limit:
    c.DockerSpawner.mem_limit = mem_limit

cull_timeout = os.environ.get("CULL_TIMEOUT")
if cull_timeout:
    cull_every = os.environ.get("CULL_EVERY", "300")
    c.JupyterHub.services = [
        {
            "name": "idle-culler",
            "command": [
                "python",
                "-m",
                "jupyterhub_idle_culler",
                f"--timeout={cull_timeout}",
                f"--cull-every={cull_every}",
                "--cull-users",
            ],
        }
    ]
    c.JupyterHub.load_roles = [
        {
            "name": "idle-culler",
            "services": ["idle-culler"],
            "scopes": ["list:users", "read:users", "admin:users"],
        }
    ]

admin_users = os.environ.get("ADMIN_USERS", "")
if admin_users:
    c.Authenticator.admin_users = {
        user.strip() for user in admin_users.split(",") if user.strip()
    }

authorize_url = os.environ.get("OAUTH_AUTHORIZE_URL")
token_url = os.environ.get("OAUTH_TOKEN_URL")
userdata_url = os.environ.get("OAUTH_USERDATA_URL")

if authorize_url and token_url and userdata_url:
    c.JupyterHub.authenticator_class = GenericOAuthenticator
    c.GenericOAuthenticator.client_id = os.environ.get("OAUTH_CLIENT_ID")
    c.GenericOAuthenticator.client_secret = os.environ.get("OAUTH_CLIENT_SECRET")
    c.GenericOAuthenticator.authorize_url = authorize_url
    c.GenericOAuthenticator.token_url = token_url
    c.GenericOAuthenticator.userdata_url = userdata_url
    c.GenericOAuthenticator.username_key = os.environ.get(
        "OAUTH_USERNAME_KEY", "preferred_username"
    )
    hub_domain = os.environ.get("HUB_DOMAIN", "")
    if hub_domain:
        c.GenericOAuthenticator.oauth_callback_url = (
            f"https://{hub_domain}/hub/oauth_callback"
        )
else:
    allow_dummy = os.environ.get("ALLOW_DUMMY_AUTH", "false").lower() == "true"
    if allow_dummy:
        from jupyterhub.auth import DummyAuthenticator

        c.JupyterHub.authenticator_class = DummyAuthenticator
        c.DummyAuthenticator.password = os.environ.get("DUMMY_PASSWORD", "mvre")
    else:
        raise RuntimeError(
            "Missing OAuth configuration. Set OAUTH_AUTHORIZE_URL, OAUTH_TOKEN_URL, and OAUTH_USERDATA_URL."
        )
"#.trim_start()
        .to_string()
}

pub fn hub_dockerfile() -> String {
    r#"
FROM jupyterhub/jupyterhub:latest

RUN pip install --no-cache-dir dockerspawner oauthenticator jupyterhub-idle-culler

COPY jupyterhub_config.py /etc/jupyterhub/jupyterhub_config.py
"#
    .trim_start()
    .to_string()
}

pub fn user_dockerfile() -> String {
    r#"
FROM jupyter/minimal-notebook:latest

COPY requirements.txt /tmp/requirements.txt
RUN pip install --no-cache-dir -r /tmp/requirements.txt \
 && rm -rf /home/jovyan/.cache/pip
"#
    .trim_start()
    .to_string()
}

pub fn user_requirements() -> String {
    "xarray\nnetCDF4\ndask\npandas\nnumpy\nmatplotlib\nscipy\n"
        .to_string()
}

pub fn mosaic_notebook() -> String {
    r###"
{
  "cells": [
    {
      "cell_type": "markdown",
      "metadata": {},
      "source": [
        "# MoSAiC Quickstart\n",
        "\n",
        "This notebook is a starting point for exploring MoSAiC datasets.\n",
        "\n",
        "## Goals\n",
        "- Verify data access\n",
        "- Load a sample dataset\n",
        "- Run a quick visualization\n"
      ]
    },
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": [
        "import os\n",
        "import xarray as xr\n",
        "import matplotlib.pyplot as plt\n",
        "\n",
        "DATA_ROOT = os.environ.get('MOSAIC_DATA', '/data/mosaic')\n",
        "print('Dataset root:', DATA_ROOT)\n"
      ]
    },
    {
      "cell_type": "code",
      "execution_count": null,
      "metadata": {},
      "outputs": [],
      "source": [
        "print('Listing data root (first 20):')\n",
        "for idx, name in enumerate(sorted(os.listdir(DATA_ROOT))):\n",
        "    print('-', name)\n",
        "    if idx >= 19:\n",
        "        break\n"
      ]
    }
  ],
  "metadata": {
    "kernelspec": {
      "display_name": "Python 3",
      "language": "python",
      "name": "python3"
    },
    "language_info": {
      "name": "python",
      "version": "3.x"
    }
  },
  "nbformat": 4,
  "nbformat_minor": 5
}
"###
    .trim_start()
    .to_string()
}

pub fn mosaic_readme() -> String {
    r#"
MoSAiC Notebook Bundle
======================

This folder contains a minimal starter notebook for MoSAiC data access.

Notebook:
- `mosaic_quickstart.ipynb`

Data mount:
- Dataset is expected at `/data/mosaic` inside the notebook container.
"#
    .trim_start()
    .to_string()
}
