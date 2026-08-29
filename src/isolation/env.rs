use std::collections::HashMap;
use std::path::Path;

pub struct HermeticEnvironmentSanitizer;

impl HermeticEnvironmentSanitizer {
    pub fn sanitize(
        raw_env: &HashMap<String, String>,
        whitelisted_keys: &[String],
        tmp_dir: Option<&Path>,
    ) -> HashMap<String, String> {
        let mut clean = HashMap::with_capacity(whitelisted_keys.len() + 12);
        for (k, v) in raw_env {
            if whitelisted_keys.contains(k) || k.starts_with("FISH_") || k.starts_with("APPLE_") {
                clean.insert(k.clone(), v.clone());
            }
        }
        let tmp = tmp_dir
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| std::env::temp_dir().to_string_lossy().into_owned());
        clean.insert("TMPDIR".to_string(), tmp.clone());
        clean.insert("TEMP".to_string(), tmp.clone());
        clean.insert("TMP".to_string(), tmp);
        clean
            .entry("TZ".to_string())
            .or_insert_with(|| "UTC".to_string());
        clean
            .entry("LC_ALL".to_string())
            .or_insert_with(|| "C".to_string());
        clean
            .entry("LANG".to_string())
            .or_insert_with(|| "C".to_string());
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_sanitizer_whitelisting_and_defaults() {
        let mut raw = HashMap::new();
        raw.insert("SECRET_TOKEN".to_string(), "leaked".to_string());
        raw.insert("PATH".to_string(), "/usr/bin".to_string());
        raw.insert("APPLE_TEST_KEY".to_string(), "sandbox".to_string());
        raw.insert("FISH_BUILD_ID".to_string(), "42".to_string());

        let whitelisted = vec!["PATH".to_string()];
        let clean = HermeticEnvironmentSanitizer::sanitize(
            &raw,
            &whitelisted,
            Some(Path::new("/custom/tmp")),
        );

        assert_eq!(clean.get("PATH"), Some(&"/usr/bin".to_string()));
        assert_eq!(clean.get("APPLE_TEST_KEY"), Some(&"sandbox".to_string()));
        assert_eq!(clean.get("FISH_BUILD_ID"), Some(&"42".to_string()));
        assert_eq!(clean.get("SECRET_TOKEN"), None);
        assert_eq!(clean.get("TMPDIR"), Some(&"/custom/tmp".to_string()));
        assert_eq!(clean.get("TZ"), Some(&"UTC".to_string()));
        assert_eq!(clean.get("LC_ALL"), Some(&"C".to_string()));
    }
}
