use apple::isolation::{
    HermeticEnvironmentSanitizer, HermeticFilesystemManager, NetworkIsolationController,
};
use apple::protocol::{IsolationLevel, MountKind, MountRule, SandboxProfile};
use apple::{AppleClient, AppleDaemonServer, ExecutionRequest, SandboxMonitor};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[test]
fn test_sandbox_profile_defaults() {
    let profile = SandboxProfile::default();
    assert_eq!(profile.name, "default-hermetic");
    assert_eq!(profile.level, IsolationLevel::StrictFilesystem);
    assert!(!profile.allow_network);
    assert!(profile.whitelisted_env.contains(&"PATH".to_string()));
}

#[test]
fn test_environment_sanitizer_scrubs_polluting_keys() {
    let mut raw = HashMap::new();
    raw.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    raw.insert("USER".to_string(), "malicious_user".to_string());
    raw.insert("SECRET_KEY".to_string(), "leaked_token".to_string());
    raw.insert("FISH_BUILD_TAG".to_string(), "v1.0".to_string());

    let whitelist = vec!["PATH".to_string()];
    let clean = HermeticEnvironmentSanitizer::sanitize(&raw, &whitelist);

    assert!(clean.contains_key("PATH"));
    assert!(clean.contains_key("FISH_BUILD_TAG"));
    assert!(!clean.contains_key("USER"));
    assert!(!clean.contains_key("SECRET_KEY"));
    assert_eq!(clean.get("TEMP").unwrap(), ".apple-scratch/tmp");
}

#[test]
fn test_network_isolation_injects_blackhole_proxies() {
    let mut env = HashMap::new();
    NetworkIsolationController::apply_network_policy(&mut env, false);

    assert_eq!(env.get("CARGO_NET_OFFLINE").unwrap(), "true");
    assert_eq!(env.get("GOPROXY").unwrap(), "off");
    assert_eq!(env.get("http_proxy").unwrap(), "http://127.0.0.1:0");
}

#[test]
fn test_monitor_detects_unauthorized_path_access() {
    let allowed = vec![PathBuf::from("/workspace/src")];
    let monitor = SandboxMonitor::new(allowed);

    let v1 = monitor.inspect_access(Path::new("/workspace/src/main.rs"), false);
    assert!(v1.is_none());

    let v2 = monitor.inspect_access(Path::new("/etc/shadow"), false);
    assert!(v2.is_some());
    assert_eq!(monitor.violation_count(), 1);
}

#[tokio::test]
async fn test_apple_daemon_ping_and_client_roundtrip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let server = Arc::new(AppleDaemonServer::new(temp_dir.path().to_path_buf()));
    let client = AppleClient::new(server);

    let is_alive = client.ping().await;
    assert!(is_alive);
}
