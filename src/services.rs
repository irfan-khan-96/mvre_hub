use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use console::style;

use crate::{
    cli::CleanOptions,
    config::{self, AppConfig},
    systemd,
    util,
};

pub fn start(config_path: &Path, app_config: &AppConfig) -> Result<()> {
    let deploy_dir = resolve_deploy_dir(app_config)?;
    run_compose(&deploy_dir, &["build", "jupyterhub", "user-image"])
        .context("failed to build images")?;
    run_compose(&deploy_dir, &["up", "-d", "jupyterhub", "traefik"])
        .context("failed to start services")?;

    println!("{}", style("Drift engaged").green());
    println!("Using deployment at {}", style(deploy_dir.display()).dim());

    let mut updated = app_config.clone();
    updated.last_deploy_dir = Some(deploy_dir);
    config::save(config_path, &updated)?;

    Ok(())
}

pub fn stop(config_path: &Path, app_config: &AppConfig) -> Result<()> {
    let deploy_dir = resolve_deploy_dir(app_config)?;
    run_compose(&deploy_dir, &["down"]).context("failed to stop services")?;

    println!("{}", style("Drift paused").yellow());
    println!("Using deployment at {}", style(deploy_dir.display()).dim());

    let mut updated = app_config.clone();
    updated.last_deploy_dir = Some(deploy_dir);
    config::save(config_path, &updated)?;

    Ok(())
}

pub fn clean(opts: CleanOptions, config_path: &Path, app_config: &AppConfig) -> Result<()> {
    if !opts.full_ice {
        anyhow::bail!("Safety lock engaged. Use --full-ice to confirm cleanup");
    }

    let deploy_dir = resolve_deploy_dir(app_config)?;

    run_compose(&deploy_dir, &["down", "-v", "--rmi", "all"])
        .context("failed to stop services before cleanup")?;
    std::fs::remove_dir_all(&deploy_dir).with_context(|| format!("failed to remove {}", deploy_dir.display()))?;

    if util::is_root() {
        let _ = systemd::remove_service();
    }

    println!("{}", style("Environment cleared").cyan());

    let mut updated = app_config.clone();
    updated.last_deploy_dir = None;
    config::save(config_path, &updated)?;

    Ok(())
}

pub fn status(app_config: &AppConfig) -> Result<()> {
    let deploy_dir = resolve_deploy_dir(app_config)?;
    let output = Command::new("docker-compose")
        .args(["ps"])
        .current_dir(&deploy_dir)
        .output()
        .context("failed to query docker-compose status")?;

    if output.status.success() {
        println!("{}", style("Current status").cyan().bold());
        println!("{}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    } else {
        anyhow::bail!(
            "docker-compose status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn run_compose(deploy_dir: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("docker-compose")
        .args(args)
        .current_dir(deploy_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to invoke docker-compose")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("docker-compose exited with status {}", status)
    }
}

fn resolve_deploy_dir(app_config: &AppConfig) -> Result<PathBuf> {
    if let Some(path) = &app_config.last_deploy_dir {
        return Ok(path.clone());
    }

    let default = PathBuf::from("./mvre-hub");
    if default.exists() {
        return Ok(default);
    }

    anyhow::bail!("Deployment not found. Run 'mvre-hub deploy' first.")
}
