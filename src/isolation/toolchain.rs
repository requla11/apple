use std::collections::HashMap;
use std::path::Path;

pub struct HermeticToolchainSanitizer;

impl HermeticToolchainSanitizer {
    pub fn inject_deterministic_flags(env: &mut HashMap<String, String>, jail_path: Option<&Path>) {
        env.insert("SOURCE_DATE_EPOCH".to_string(), "0".to_string());
        env.insert("ZERO_AR_DATE".to_string(), "1".to_string());
        env.insert("FORCE_SOURCE_DATE".to_string(), "1".to_string());
        env.insert("PYTHONHASHSEED".to_string(), "0".to_string());
        env.insert("TZ".to_string(), "UTC".to_string());
        env.insert("LC_ALL".to_string(), "C".to_string());
        env.insert("LANG".to_string(), "C".to_string());

        if let Some(jail) = jail_path {
            let jail_str = jail.to_string_lossy().into_owned();
            let home_dir = jail.join("home");
            let _ = std::fs::create_dir_all(&home_dir);

            env.insert("HOME".to_string(), home_dir.to_string_lossy().into_owned());
            env.insert(
                "USERPROFILE".to_string(),
                home_dir.to_string_lossy().into_owned(),
            );
            env.insert(
                "APPDATA".to_string(),
                home_dir
                    .join("AppData")
                    .join("Roaming")
                    .to_string_lossy()
                    .into_owned(),
            );
            env.insert(
                "LOCALAPPDATA".to_string(),
                home_dir
                    .join("AppData")
                    .join("Local")
                    .to_string_lossy()
                    .into_owned(),
            );

            let remap_flag = format!("--remap-path-prefix={jail_str}=/apple_sandbox");
            let existing_rustflags = env.get("RUSTFLAGS").cloned().unwrap_or_default();
            if !existing_rustflags.contains("--remap-path-prefix") {
                if existing_rustflags.is_empty() {
                    env.insert("RUSTFLAGS".to_string(), remap_flag);
                } else {
                    env.insert(
                        "RUSTFLAGS".to_string(),
                        format!("{existing_rustflags} {remap_flag}"),
                    );
                }
            }

            let cflags = format!(
                "-ffile-prefix-map={jail_str}=/apple_sandbox -fdebug-prefix-map={jail_str}=/apple_sandbox -Wno-builtin-macro-redefined -D__DATE__=\"\" -D__TIME__=\"\" -D__TIMESTAMP__=\"\""
            );
            let existing_cflags = env.get("CFLAGS").cloned().unwrap_or_default();
            if !existing_cflags.contains("-ffile-prefix-map") {
                if existing_cflags.is_empty() {
                    env.insert("CFLAGS".to_string(), cflags.clone());
                    env.insert("CXXFLAGS".to_string(), cflags);
                } else {
                    env.insert("CFLAGS".to_string(), format!("{existing_cflags} {cflags}"));
                    env.insert(
                        "CXXFLAGS".to_string(),
                        format!("{existing_cflags} {cflags}"),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_inject_deterministic_flags() {
        let temp = TempDir::new().unwrap();
        let mut env = HashMap::new();
        HermeticToolchainSanitizer::inject_deterministic_flags(&mut env, Some(temp.path()));

        assert_eq!(env.get("SOURCE_DATE_EPOCH"), Some(&"0".to_string()));
        assert_eq!(env.get("ZERO_AR_DATE"), Some(&"1".to_string()));
        assert_eq!(env.get("PYTHONHASHSEED"), Some(&"0".to_string()));
        assert_eq!(env.get("TZ"), Some(&"UTC".to_string()));
        assert!(env.get("HOME").is_some());
        assert!(
            env.get("RUSTFLAGS")
                .unwrap()
                .contains("--remap-path-prefix")
        );
        assert!(env.get("CFLAGS").unwrap().contains("-ffile-prefix-map"));
    }
}
