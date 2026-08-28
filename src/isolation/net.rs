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
            env.insert("NO_PROXY".to_string(), "".to_string());
            env.insert("CARGO_NET_OFFLINE".to_string(), "true".to_string());
            env.insert("GOPROXY".to_string(), "off".to_string());
            env.insert("PIP_NO_INDEX".to_string(), "1".to_string());
            env.insert("NPM_CONFIG_OFFLINE".to_string(), "true".to_string());
        }
    }
}
