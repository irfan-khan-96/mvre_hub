use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logging(verbosity: u8) {
    let filter = match verbosity {
        0 => EnvFilter::new("info"),
        1 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };

    let _ = fmt().with_env_filter(filter).try_init();
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().context("missing parent directory for atomic write")?;
    let mut tmp_path = PathBuf::from(dir);
    tmp_path.push(format!(".{}.tmp", uuid_segment()));

    {
        let mut file = File::create(&tmp_path).with_context(|| format!("failed to create temp file {}", tmp_path.display()))?;
        file.write_all(bytes).with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
        file.sync_all().ok();
    }

    fs::rename(&tmp_path, path).with_context(|| format!("failed to rename temp file to {}", path.display()))?;
    Ok(())
}

fn uuid_segment() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}", nanos)
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("failed to create dir {}", path.display()))
}

pub fn validate_non_empty(name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("{} must not be empty", name)
    }
    Ok(())
}

pub fn prompt_or_use(default: Option<String>, prompt: &str, allow_empty: bool) -> Result<String> {
    use dialoguer::{Input, theme::ColorfulTheme};
    let theme = ColorfulTheme::default();

    let mut input = Input::with_theme(&theme);
    input.with_prompt(prompt);
    if let Some(value) = default {
        input.default(value);
    }
    let value: String = input.interact_text()?;
    if !allow_empty {
        validate_non_empty(prompt, &value)?;
    }
    Ok(value)
}

pub fn path_display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
}

pub fn write_string(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    atomic_write(path, contents.as_bytes()).with_context(|| format!("failed to write {}", path.display()))
}

pub fn set_file_mode(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(mode);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

pub fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        nix::unistd::Uid::effective().is_root()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn maybe_symlink(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(from, to)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(from, to)
    }
}
