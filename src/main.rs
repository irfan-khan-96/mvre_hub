// src/main.rs - MVRE-Hub Core Implementation
use std::{
    fs, path::Path,
    process::{Command, Stdio},
};
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use anyhow::{Context, Result};
use console::style;
use nix::unistd::{Uid, Gid};
use clap::{Parser, Subcommand};

const DOCKER_COMPOSE_TEMPLATE: &str = r#"services:
  jupyterhub:
    image: jupyterhub/jupyterhub
    env_file: .env
    volumes:
      - ./user_data:/srv/jupyterhub
      - ./traefik/certs:/certs
    labels:
      - "traefik.http.routers.jupyterhub.rule=Host(`{DOMAIN}`)"
      - "traefik.http.routers.jupyterhub.entrypoints=websecure"
      - "traefik.http.routers.jupyterhub.tls.certresolver=mosaicresolver"
  
  traefik:
    image: traefik:v2.9
    command:
      - "--providers.file.filename=/etc/traefik/config.yml"
      - "--entrypoints.websecure.address=:443"
    volumes:
      - ./traefik:/etc/traefik
    ports:
      - "8080:80"
      - "8443:443"
"#;

#[derive(Parser)]
#[command(name = "mvre-hub")]
#[command(about = "MOSAiC Virtual Research Environment Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive deployment setup
    Deploy {
        /// Force overwrite existing deployment
        #[arg(short, long)]
        force: bool
    },
    /// Start JupyterHub services
    Start,
    /// Stop services (preserve data)
    Stop,
    /// Full environment cleanup
    Clean {
        /// Confirm destructive operation
        #[arg(long)]
        full_ice: bool
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let theme = ColorfulTheme {
        success_prefix: style("❄️ ".to_string()).cyan().bold(),
        error_prefix: style("⛔ ".to_string()).red().bold(),
        ..ColorfulTheme::default()
    };

    match cli.command {
        Commands::Deploy { force } => deploy(&theme, force),
        Commands::Start => start_services(),
        Commands::Stop => stop_services(),
        Commands::Clean { full_ice } => clean_environment(full_ice),
    }
}

fn deploy(theme: &ColorfulTheme, force: bool) -> Result<()> {
    println!("\n{}", style("MOSAiC Virtual Research Environment Hub").cyan().bold());
    println!("{}", style("🚀 Initial Deployment Setup").dim());

    let deploy_dir: String = Input::with_theme(theme)
        .with_prompt("Enter deployment directory")
        .default("./mvre-hub".into())
        .interact()?;

    let deploy_path = Path::new(&deploy_dir);
    
    // Handle existing deployment
    if deploy_path.exists() {
        if !force {
            anyhow::bail!("⛔ Deployment exists! Use --force to overwrite");
        }
        fs::remove_dir_all(deploy_path)?;
    }

    // Create directory structure
    fs::create_dir_all(deploy_path)?;
    fs::create_dir_all(deploy_path.join("traefik"))?;
    fs::create_dir_all(deploy_path.join("user_data"))?;

    // Set permissions
    let uid = Uid::current();
    let gid = Gid::current();
    nix::unistd::chown(deploy_path, Some(uid), Some(gid))?;

    // Collect configuration
    let domain: String = Input::with_theme(theme)
        .with_prompt("Domain name (e.g., hub.mosaic.org)")
        .interact()?;

    let client_id: String = Input::with_theme(theme)
        .with_prompt("Helmholtz AAI Client ID")
        .interact()?;

    let client_secret: String = Input::with_theme(theme)
        .with_prompt("Helmholtz AAI Client Secret")
        .interact()?;

    // Generate config files
    let compose = DOCKER_COMPOSE_TEMPLATE.replace("{DOMAIN}", &domain);
    fs::write(deploy_path.join("docker-compose.yml"), compose)
        .context("Failed to create docker-compose.yml")?;

    fs::write(
        deploy_path.join(".env"),
        format!("OAUTH_CLIENT_ID={}\nOAUTH_CLIENT_SECRET={}", client_id, client_secret),
    ).context("Failed to create .env file")?;

    // Systemd service setup
    if Confirm::with_theme(theme)
        .with_prompt("Enable auto-start on boot?")
        .default(true)
        .interact()?
    {
        if !nix::unistd::Uid::effective().is_root() {
            eprintln!("{}", style("⚠️  Root required for systemd setup").yellow());
            eprintln!("{}", style("   Run with sudo to complete this step").dim());
            return Ok(());
        }

        let service_content = format!(
            "[Unit]\nDescription=MVRE-Hub\nAfter=network.target\n\n[Service]\n\
            ExecStart=/usr/bin/docker-compose -f {}/docker-compose.yml up\n\
            ExecStop=/usr/bin/docker-compose -f {}/docker-compose.yml down\n\
            Restart=always\nUser={}\nWorkingDirectory={}\n\n[Install]\nWantedBy=multi-user.target",
            deploy_dir, deploy_dir, whoami::username(), deploy_dir
        );

        fs::write("/etc/systemd/system/mvre-hub.service", service_content)
            .context("Failed to create systemd service")?;
        
        println!("{}", style("❄️  Auto-start configured successfully").cyan());
    }

    println!("\n{}", style("Deployment Complete!").green().bold());
    println!("1. Start services: {}", style(format!("sudo mvre-hub start")).cyan());
    println!("2. Access dashboard: {}", style(format!("https://{}", domain)).cyan());
    
    Ok(())
}

fn start_services() -> Result<()> {
    let deploy_dir = find_deployment_dir()?;
    let status = Command::new("docker-compose")
        .args(["up", "-d"])
        .current_dir(deploy_dir)
        .stdout(Stdio::inherit())
        .status()?;

    if status.success() {
        println!("{}", style("🚀 Services started successfully").green());
        Ok(())
    } else {
        anyhow::bail!("⛔ Failed to start services");
    }
}

fn stop_services() -> Result<()> {
    let deploy_dir = find_deployment_dir()?;
    let status = Command::new("docker-compose")
        .args(["down"])
        .current_dir(deploy_dir)
        .stdout(Stdio::inherit())
        .status()?;

    if status.success() {
        println!("{}", style("🛑 Services stopped successfully").yellow());
        Ok(())
    } else {
        anyhow::bail!("⛔ Failed to stop services");
    }
}

fn clean_environment(full_ice: bool) -> Result<()> {
    if !full_ice {
        anyhow::bail!("⛔ Safety lock engaged! Use --full-ice to confirm glacier melt");
    }

    let deploy_dir = find_deployment_dir()?;
    
    // Stop and remove containers
    let _ = Command::new("docker-compose")
        .args(["down", "-v", "--rmi", "all"])
        .current_dir(&deploy_dir)
        .status();

    // Remove deployment directory
    fs::remove_dir_all(&deploy_dir)?;

    // Remove systemd service if root
    if nix::unistd::Uid::effective().is_root() {
        let _ = fs::remove_file("/etc/systemd/system/mvre-hub.service");
    }

    println!("{}", style("❄️ Full environment cleanup completed").cyan());
    Ok(())
}

fn find_deployment_dir() -> Result<String> {
    let default_dir = Path::new("./mvre-hub");
    if default_dir.exists() {
        Ok(default_dir.to_str().unwrap().to_string())
    } else {
        anyhow::bail!("⛔ Deployment not found. Run 'mvre-hub deploy' first")
    }
}
