use std::collections::HashMap;
use std::path::Path;

pub struct AmbientDaemonScrubber;

impl AmbientDaemonScrubber {
    pub fn scrub_ambient_env(env: &mut HashMap<String, String>) {
        let keys_to_remove = [
            "SSH_AUTH_SOCK",
            "DOCKER_HOST",
            "DOCKER_CERT_PATH",
            "DOCKER_TLS_VERIFY",
            "DBUS_SESSION_BUS_ADDRESS",
            "DBUS_SYSTEM_BUS_ADDRESS",
            "GPG_AGENT_INFO",
            "KRB5CCNAME",
            "XAUTHORITY",
            "DISPLAY",
            "WAYLAND_DISPLAY",
            "PULSE_SERVER",
            "VAULT_TOKEN",
            "KUBECONFIG",
        ];

        for key in &keys_to_remove {
            env.remove(*key);
        }
    }

    pub fn is_forbidden_ambient_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let lower = path_str.to_lowercase();

        lower.contains("docker.sock")
            || lower.contains("podman.sock")
            || lower.contains(".x11-unix")
            || lower.contains("ssh-agent")
            || lower.contains("keyring")
            || lower.contains("\\\\.\\pipe\\docker_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrub_ambient_env() {
        let mut env = HashMap::new();
        env.insert("SSH_AUTH_SOCK".to_string(), "/tmp/ssh.sock".to_string());
        env.insert(
            "DOCKER_HOST".to_string(),
            "unix:///var/run/docker.sock".to_string(),
        );
        env.insert("PATH".to_string(), "/usr/bin".to_string());

        AmbientDaemonScrubber::scrub_ambient_env(&mut env);

        assert_eq!(env.get("SSH_AUTH_SOCK"), None);
        assert_eq!(env.get("DOCKER_HOST"), None);
        assert_eq!(env.get("PATH"), Some(&"/usr/bin".to_string()));
    }

    #[test]
    fn test_is_forbidden_ambient_path() {
        assert!(AmbientDaemonScrubber::is_forbidden_ambient_path(Path::new(
            "/var/run/docker.sock"
        )));
        assert!(AmbientDaemonScrubber::is_forbidden_ambient_path(Path::new(
            "/tmp/.X11-unix/X0"
        )));
        assert!(!AmbientDaemonScrubber::is_forbidden_ambient_path(
            Path::new("/workspace/src/main.rs")
        ));
    }
}
