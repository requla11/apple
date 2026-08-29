use std::collections::HashMap;

pub struct NetworkIsolationController;

impl NetworkIsolationController {
    pub fn apply_network_policy(env: &mut HashMap<String, String>, allow_network: bool) {
        if !allow_network {
            env.insert("http_proxy".to_string(), "http://127.0.0.1:0".to_string());
            env.insert("https_proxy".to_string(), "http://127.0.0.1:0".to_string());
            env.insert("all_proxy".to_string(), "socks5://127.0.0.1:0".to_string());
            env.insert("ALL_PROXY".to_string(), "socks5://127.0.0.1:0".to_string());
            env.insert("HTTP_PROXY".to_string(), "http://127.0.0.1:0".to_string());
            env.insert("HTTPS_PROXY".to_string(), "http://127.0.0.1:0".to_string());
            env.insert("FTP_PROXY".to_string(), "http://127.0.0.1:0".to_string());
            env.insert("NO_PROXY".to_string(), "".to_string());

            env.insert(
                "GIT_CONFIG_PARAMETERS".to_string(),
                "'http.proxy=http://127.0.0.1:0'".to_string(),
            );
            env.insert(
                "CURL_OPT_PROXY".to_string(),
                "http://127.0.0.1:0".to_string(),
            );

            env.insert("CARGO_NET_OFFLINE".to_string(), "true".to_string());
            env.insert("GOPROXY".to_string(), "off".to_string());
            env.insert("PIP_NO_INDEX".to_string(), "1".to_string());
            env.insert("NPM_CONFIG_OFFLINE".to_string(), "true".to_string());
            env.insert("YARN_OFFLINE".to_string(), "true".to_string());
            env.insert("PNPM_OFFLINE".to_string(), "true".to_string());
            env.insert("MAVEN_OPTS".to_string(), "-Dmaven.offline=true".to_string());
            env.insert(
                "GRADLE_OPTS".to_string(),
                "-Dorg.gradle.offline=true".to_string(),
            );
            env.insert("DOTNET_RESTORE_OFFLINE".to_string(), "true".to_string());
            env.insert(
                "SWIFT_PACKAGE_COLLECTIONS_ONLINE".to_string(),
                "false".to_string(),
            );
            env.insert("PUB_CACHE_OFFLINE".to_string(), "true".to_string());

            env.remove("GITHUB_TOKEN");
            env.remove("GH_TOKEN");
            env.remove("NPM_TOKEN");
            env.remove("CARGO_REGISTRY_TOKEN");
            env.remove("AWS_ACCESS_KEY_ID");
            env.remove("AWS_SECRET_ACCESS_KEY");
            env.remove("AWS_SESSION_TOKEN");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_isolation_policy_applied_when_disallowed() {
        let mut env = HashMap::new();
        env.insert("GITHUB_TOKEN".to_string(), "secret123".to_string());
        NetworkIsolationController::apply_network_policy(&mut env, false);

        assert_eq!(env.get("CARGO_NET_OFFLINE"), Some(&"true".to_string()));
        assert_eq!(env.get("GOPROXY"), Some(&"off".to_string()));
        assert_eq!(env.get("PIP_NO_INDEX"), Some(&"1".to_string()));
        assert_eq!(env.get("NPM_CONFIG_OFFLINE"), Some(&"true".to_string()));
        assert_eq!(env.get("DOTNET_RESTORE_OFFLINE"), Some(&"true".to_string()));
        assert_eq!(
            env.get("http_proxy"),
            Some(&"http://127.0.0.1:0".to_string())
        );
        assert_eq!(env.get("GITHUB_TOKEN"), None);
    }

    #[test]
    fn test_network_isolation_policy_noop_when_allowed() {
        let mut env = HashMap::new();
        env.insert("GITHUB_TOKEN".to_string(), "secret123".to_string());
        NetworkIsolationController::apply_network_policy(&mut env, true);
        assert_eq!(env.get("GITHUB_TOKEN"), Some(&"secret123".to_string()));
        assert_eq!(env.get("CARGO_NET_OFFLINE"), None);
    }
}
