use std::{
    fs,
    path::Path,
};

use anyhow::{Context, Result};

use crate::util;

const SERVICE_PATH: &str = "/etc/systemd/system/mvre-hub.service";

pub fn install_service(deploy_dir: &Path) -> Result<()> {
    let user = whoami::username();
    let content = format!(
        "[Unit]\nDescription=MVRE-Hub\nAfter=network.target\n\n[Service]\n\
ExecStart=/usr/bin/env docker-compose -f {}/docker-compose.yml up\n\
ExecStop=/usr/bin/env docker-compose -f {}/docker-compose.yml down\n\
Restart=always\nUser={}\nWorkingDirectory={}\n\n[Install]\nWantedBy=multi-user.target\n",
        deploy_dir.display(),
        deploy_dir.display(),
        user,
        deploy_dir.display(),
    );

    util::atomic_write(Path::new(SERVICE_PATH), content.as_bytes())
        .with_context(|| format!("failed to write {}", SERVICE_PATH))?;

    reload_systemd().context("failed to reload systemd")?;
    enable_service().context("failed to enable systemd service")?;

    Ok(())
}

pub fn remove_service() -> Result<()> {
    if Path::new(SERVICE_PATH).exists() {
        fs::remove_file(SERVICE_PATH).with_context(|| format!("failed to remove {}", SERVICE_PATH))?;
    }
    Ok(())
}

fn reload_systemd() -> Result<()> {
    let status = std::process::Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("failed to run systemctl daemon-reload")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("systemctl daemon-reload failed: {}", status)
    }
}

fn enable_service() -> Result<()> {
    let status = std::process::Command::new("systemctl")
        .args(["enable", "mvre-hub"])
        .status()
        .context("failed to run systemctl enable")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("systemctl enable failed: {}", status)
    }
}
