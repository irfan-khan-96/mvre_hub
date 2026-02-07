use std::path::PathBuf;

use mvre_hub::config::{self, AppConfig};

#[test]
fn config_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let path = config::resolve_config_path().expect("resolve config path");

    let mut cfg = AppConfig::default();
    cfg.last_deploy_dir = Some(PathBuf::from("/tmp/mvre"));
    cfg.last_domain = Some("hub.example.org".to_string());

    config::save(&path, &cfg).expect("save");
    let loaded = config::load().expect("load");

    assert_eq!(loaded.last_deploy_dir, cfg.last_deploy_dir);
    assert_eq!(loaded.last_domain, cfg.last_domain);
}
