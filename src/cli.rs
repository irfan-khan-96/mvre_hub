use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "mvre-hub")]
#[command(about = "MVRE Polar Drift Hub Manager", long_about = None)]
pub struct Cli {
    /// Increase logging verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Interactive deployment setup
    Deploy {
        #[command(flatten)]
        opts: DeployOptions,
    },
    /// Start JupyterHub services
    Start,
    /// Stop services (preserve data)
    Stop,
    /// Full environment cleanup
    Clean {
        #[command(flatten)]
        opts: CleanOptions,
    },
    /// Show deployment status
    Status,
}

#[derive(Args, Debug, Clone)]
pub struct DeployOptions {
    /// Force overwrite existing deployment
    #[arg(short, long)]
    pub force: bool,

    /// Domain name for the hub (e.g., hub.example.org)
    #[arg(long)]
    pub domain: Option<String>,

    /// ACME email for TLS certificates
    #[arg(long)]
    pub acme_email: Option<String>,

    /// Helmholtz AAI Client ID
    #[arg(long)]
    pub client_id: Option<String>,

    /// Helmholtz AAI Client Secret
    #[arg(long)]
    pub client_secret: Option<String>,

    /// Host path to MoSAiC dataset (required)
    #[arg(long)]
    pub dataset_path: Option<String>,

    /// Allow deployment if dataset path is missing (testing only)
    #[arg(long)]
    pub allow_missing_dataset: bool,

    /// Install bundled MoSAiC notebooks into shared path
    #[arg(long)]
    pub install_notebooks: bool,

    /// Enable production profile (Postgres + culling + limits)
    #[arg(long)]
    pub production: bool,

    /// Skip systemd setup
    #[arg(long)]
    pub no_systemd: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CleanOptions {
    /// Confirm destructive operation
    #[arg(long)]
    pub full_ice: bool,
}
