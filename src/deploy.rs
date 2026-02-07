use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Password, theme::ColorfulTheme};

use crate::{
    cli::DeployOptions,
    config::{self, AppConfig},
    systemd,
    templates,
    util,
};

const BANNER: &str = r#"
    __/___
  _____/_____
  \_POLARSTERN/
~~~\_________/~~~
"#;

#[derive(Debug)]
struct DeployInputs {
    domain: String,
    acme_email: String,
    client_id: String,
    client_secret: String,
    dataset_path: String,
    dataset_mount: String,
    shared_path: Option<String>,
    shared_mount: String,
    admin_users: Option<String>,
    user_image: String,
    oauth_authorize_url: Option<String>,
    oauth_token_url: Option<String>,
    oauth_userdata_url: Option<String>,
    oauth_username_key: String,
    install_notebooks: bool,
    allow_missing_dataset: bool,
    production: bool,
    db_user: String,
    db_name: String,
    db_password: String,
    db_host: String,
    db_port: u16,
    cpu_limit: Option<String>,
    mem_limit: Option<String>,
    cull_timeout: Option<u64>,
    cull_every: Option<u64>,
}

pub fn run(
    opts: DeployOptions,
    config_path: &Path,
    app_config: &mut AppConfig,
) -> Result<()> {
    println!("\n{}", style(BANNER).cyan());
    println!("{}", style("MVRE Polar Drift Hub").cyan().bold());
    println!("{}", style("Arctic Mission Deployment Route").dim());

    let deploy_dir = resolve_deploy_dir(opts.force, app_config.last_deploy_dir.clone())?;
    let inputs = collect_inputs(&opts, app_config.last_domain.clone())?;

    create_dirs(&deploy_dir, inputs.shared_path.is_some())?;
    write_configs(&deploy_dir, &inputs)?;
    chown_dir(&deploy_dir)?;

    app_config.last_deploy_dir = Some(deploy_dir.clone());
    app_config.last_domain = Some(inputs.domain.clone());
    config::save(config_path, app_config)?;

    if !opts.no_systemd {
        maybe_setup_systemd(&deploy_dir)?;
    }

    println!("\n{}", style("Drift Established").green().bold());
    println!("1. Start services: {}", style("mvre-hub start").cyan());
    println!("2. Access hub: {}", style(format!("https://{}", inputs.domain)).cyan());

    Ok(())
}

fn resolve_deploy_dir(force: bool, default: Option<PathBuf>) -> Result<PathBuf> {
    let default_dir = default.unwrap_or_else(|| PathBuf::from("./mvre-hub"));
    let deploy_dir = util::prompt_or_use(
        Some(util::path_display(&default_dir)),
        "Enter deployment directory",
        false,
    )?;

    let deploy_path = PathBuf::from(deploy_dir);

    if deploy_path.exists() {
        if !force {
            anyhow::bail!("Deployment exists. Use --force to overwrite.");
        }
        fs::remove_dir_all(&deploy_path).with_context(|| format!("failed to remove {}", deploy_path.display()))?;
    }

    Ok(deploy_path)
}

fn collect_inputs(opts: &DeployOptions, default_domain: Option<String>) -> Result<DeployInputs> {
    let domain = match &opts.domain {
        Some(value) => value.clone(),
        None => util::prompt_or_use(default_domain, "Domain name (e.g., hub.example.org)", false)?,
    };

    let acme_email = match &opts.acme_email {
        Some(value) => value.clone(),
        None => util::prompt_or_use(None, "ACME email (for TLS)", false)?,
    };

    let client_id = match &opts.client_id {
        Some(value) => value.clone(),
        None => util::prompt_or_use(None, "Helmholtz AAI Client ID", false)?,
    };

    let client_secret = match &opts.client_secret {
        Some(value) => value.clone(),
        None => util::prompt_or_use(None, "Helmholtz AAI Client Secret", false)?,
    };

    let dataset_path = match &opts.dataset_path {
        Some(value) => value.clone(),
        None => util::prompt_or_use(None, "MoSAiC dataset host path", false)?,
    };

    let mut shared_path = {
        let prompt = "Shared notebooks host path (optional)";
        let value = util::prompt_or_use(None, prompt, true)?;
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    };

    if opts.install_notebooks && shared_path.is_none() {
        shared_path = Some("./shared".to_string());
    }

    let admin_users = {
        let value = util::prompt_or_use(None, "Admin users (comma-separated, optional)", true)?;
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    };

    let oauth_authorize_url = Some(util::prompt_or_use(None, "OAuth authorize URL", false)?);
    let oauth_token_url = Some(util::prompt_or_use(None, "OAuth token URL", false)?);
    let oauth_userdata_url = Some(util::prompt_or_use(None, "OAuth userinfo URL", false)?);

    let production = opts.production;
    let db_user = if production { "mvre".to_string() } else { "".to_string() };
    let db_name = if production { "mvre_hub".to_string() } else { "".to_string() };
    let db_host = if production { "postgres".to_string() } else { "".to_string() };
    let db_port = 5432;

    let db_password = if production {
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Postgres password")
            .interact()?
    } else {
        String::new()
    };

    let cpu_limit = if production { Some("2".to_string()) } else { None };
    let mem_limit = if production { Some("4G".to_string()) } else { None };
    let cull_timeout = if production { Some(3600) } else { None };
    let cull_every = if production { Some(300) } else { None };

    Ok(DeployInputs {
        domain,
        acme_email,
        client_id,
        client_secret,
        dataset_path,
        dataset_mount: "/data/mosaic".to_string(),
        shared_path,
        shared_mount: "/home/jovyan/shared".to_string(),
        admin_users,
        user_image: "mvre-user:latest".to_string(),
        oauth_authorize_url,
        oauth_token_url,
        oauth_userdata_url,
        oauth_username_key: "preferred_username".to_string(),
        install_notebooks: opts.install_notebooks,
        allow_missing_dataset: opts.allow_missing_dataset,
        production,
        db_user,
        db_name,
        db_password,
        db_host,
        db_port,
        cpu_limit,
        mem_limit,
        cull_timeout,
        cull_every,
    })
}

fn create_dirs(deploy_path: &Path, needs_shared: bool) -> Result<()> {
    util::ensure_dir(deploy_path)?;
    util::ensure_dir(&deploy_path.join("traefik"))?;
    util::ensure_dir(&deploy_path.join("hub"))?;
    util::ensure_dir(&deploy_path.join("user"))?;
    if needs_shared {
        util::ensure_dir(&deploy_path.join("shared"))?;
    }
    util::ensure_dir(&deploy_path.join("jupyterhub_data"))?;
    Ok(())
}

fn write_configs(deploy_path: &Path, inputs: &DeployInputs) -> Result<()> {
    let compose = templates::docker_compose(&inputs.domain, &inputs.acme_email, inputs.production);
    util::write_string(&deploy_path.join("docker-compose.yml"), &compose)?;

    let shared_host = inputs
        .shared_path
        .as_ref()
        .map(|value| resolve_host_path(deploy_path, value));

    let dataset_host = resolve_host_path(deploy_path, &inputs.dataset_path);
    validate_dataset_path(&dataset_host, inputs.allow_missing_dataset, deploy_path)?;

    if let Some(shared_path) = &shared_host {
        let path = Path::new(shared_path);
        if !path.exists() {
            util::ensure_dir(path)?;
        }
    }

    let env = templates::env_file(&templates::EnvValues {
        client_id: &inputs.client_id,
        client_secret: &inputs.client_secret,
        domain: &inputs.domain,
        user_image: &inputs.user_image,
        dataset_host: &dataset_host,
        dataset_mount: &inputs.dataset_mount,
        allow_missing_dataset: inputs.allow_missing_dataset,
        shared_host: shared_host.as_deref(),
        shared_mount: &inputs.shared_mount,
        admin_users: inputs.admin_users.as_deref(),
        oauth_authorize_url: inputs.oauth_authorize_url.as_deref(),
        oauth_token_url: inputs.oauth_token_url.as_deref(),
        oauth_userdata_url: inputs.oauth_userdata_url.as_deref(),
        oauth_username_key: &inputs.oauth_username_key,
        production: inputs.production,
        db_user: &inputs.db_user,
        db_name: &inputs.db_name,
        db_password: &inputs.db_password,
        db_host: &inputs.db_host,
        db_port: inputs.db_port,
        cpu_limit: inputs.cpu_limit.as_deref(),
        mem_limit: inputs.mem_limit.as_deref(),
        cull_timeout: inputs.cull_timeout,
        cull_every: inputs.cull_every,
    });
    let env_path = deploy_path.join(".env");
    util::write_string(&env_path, &env)?;
    util::set_file_mode(&env_path, 0o600).ok();

    let certs = deploy_path.join("traefik").join("acme.json");
    if !certs.exists() {
        util::write_string(&certs, "{}")?;
    }
    util::set_file_mode(&certs, 0o600).ok();

    let hub_dir = deploy_path.join("hub");
    util::write_string(&hub_dir.join("jupyterhub_config.py"), &templates::jupyterhub_config())?;
    util::write_string(&hub_dir.join("Dockerfile"), &templates::hub_dockerfile())?;

    let user_dir = deploy_path.join("user");
    util::write_string(&user_dir.join("Dockerfile"), &templates::user_dockerfile())?;
    util::write_string(&user_dir.join("requirements.txt"), &templates::user_requirements())?;

    if inputs.install_notebooks {
        let target = shared_host
            .clone()
            .unwrap_or_else(|| deploy_path.join("shared").to_string_lossy().to_string());
        write_mosaic_bundle(Path::new(&target))?;
    }

    Ok(())
}

fn resolve_host_path(deploy_path: &Path, value: &str) -> String {
    let path = Path::new(value);
    if path.is_absolute() {
        return value.to_string();
    }
    deploy_path.join(value).to_string_lossy().to_string()
}

fn validate_dataset_path(value: &str, allow_missing: bool, deploy_path: &Path) -> Result<()> {
    let path = Path::new(value);
    if !path.exists() {
        if allow_missing {
            if value.starts_with(&*deploy_path.to_string_lossy()) {
                util::ensure_dir(path)?;
            } else {
                eprintln!(
                    "{}",
                    style(format!(
                        "Warning: dataset path not found (testing mode): {}",
                        path.display()
                    ))
                    .yellow()
                );
            }
            return Ok(());
        }
        anyhow::bail!("Dataset path does not exist: {}", path.display());
    }
    Ok(())
}

fn write_mosaic_bundle(target: &Path) -> Result<()> {
    util::ensure_dir(target)?;
    util::write_string(&target.join("README.txt"), &templates::mosaic_readme())?;
    util::write_string(
        &target.join("mosaic_quickstart.ipynb"),
        &templates::mosaic_notebook(),
    )?;
    Ok(())
}

fn chown_dir(deploy_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let uid = nix::unistd::Uid::current();
        let gid = nix::unistd::Gid::current();
        nix::unistd::chown(deploy_path, Some(uid), Some(gid))?;
    }
    Ok(())
}

fn maybe_setup_systemd(deploy_path: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let enable = Confirm::with_theme(&theme)
        .with_prompt("Enable auto-start on boot?")
        .default(true)
        .interact()?;

    if !enable {
        return Ok(());
    }

    if !util::is_root() {
        eprintln!("{}", style("Root required for systemd setup").yellow());
        eprintln!("{}", style("Run with sudo to complete this step").dim());
        return Ok(());
    }

    systemd::install_service(deploy_path)?;
    println!("{}", style("Auto-start configured").cyan());

    Ok(())
}
