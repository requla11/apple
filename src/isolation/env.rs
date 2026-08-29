use std::collections::HashMap;

pub struct HermeticEnvironmentSanitizer;

impl HermeticEnvironmentSanitizer {
    pub fn sanitize(
        raw_env: &HashMap<String, String>,
        whitelisted_keys: &[String],
    ) -> HashMap<String, String> {
        let mut clean = HashMap::with_capacity(whitelisted_keys.len() + 8);
        for (k, v) in raw_env {
            if whitelisted_keys.contains(k) || k.starts_with("FISH_") || k.starts_with("APPLE_") {
                clean.insert(k.clone(), v.clone());
            }
        }
        clean.insert("TMPDIR".to_string(), ".apple-scratch/tmp".to_string());
        clean.insert("TEMP".to_string(), ".apple-scratch/tmp".to_string());
        clean.insert("TMP".to_string(), ".apple-scratch/tmp".to_string());
        clean
    }
}
