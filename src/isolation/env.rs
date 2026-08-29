use std::collections::HashMap;
use std::path::Path;

pub struct HermeticEnvironmentSanitizer;

impl HermeticEnvironmentSanitizer {
    /// Keep only whitelisted variables (plus FISH_/APPLE_ prefixed ones) and
    /// point TMPDIR/TEMP/TMP at the sandbox scratch directory. When
    /// `tmp_dir` is `None` the process-default temp directory is used.
    pub fn sanitize(
        raw_env: &HashMap<String, String>,
        whitelisted_keys: &[String],
        tmp_dir: Option<&Path>,
    ) -> HashMap<String, String> {
        let mut clean = HashMap::with_capacity(whitelisted_keys.len() + 8);
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
    }
}
