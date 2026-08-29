use std::path::{Path, PathBuf};

pub struct SeatbeltProfileBuilder;

impl SeatbeltProfileBuilder {
    pub fn build_sbpl_profile(
        jail_path: &Path,
        read_paths: &[PathBuf],
        write_paths: &[PathBuf],
        allow_network: bool,
    ) -> String {
        let mut sbpl = String::from("(version 1)\n(deny default)\n");
        sbpl.push_str("(allow process-exec\n");
        sbpl.push_str("    (literal \"/bin/sh\")\n");
        sbpl.push_str("    (literal \"/bin/bash\")\n");
        sbpl.push_str("    (literal \"/bin/zsh\")\n");
        sbpl.push_str("    (literal \"/usr/bin/env\")\n");
        sbpl.push_str("    (subpath \"/usr/bin\")\n");
        sbpl.push_str("    (subpath \"/usr/local/bin\")\n");
        sbpl.push_str("    (subpath \"/opt/homebrew/bin\"))\n");

        sbpl.push_str("(allow file-read*\n");
        sbpl.push_str("    (literal \"/dev/null\")\n");
        sbpl.push_str("    (literal \"/dev/urandom\")\n");
        sbpl.push_str("    (literal \"/dev/random\")\n");
        sbpl.push_str("    (subpath \"/usr/lib\")\n");
        sbpl.push_str("    (subpath \"/usr/share\")\n");
        sbpl.push_str("    (subpath \"/Library/Developer\")\n");
        sbpl.push_str(&format!("    (subpath \"{}\")\n", jail_path.display()));
        for p in read_paths {
            sbpl.push_str(&format!("    (subpath \"{}\")\n", p.display()));
        }
        sbpl.push_str(")\n");

        sbpl.push_str("(allow file-write*\n");
        sbpl.push_str("    (literal \"/dev/null\")\n");
        sbpl.push_str(&format!("    (subpath \"{}/tmp\")\n", jail_path.display()));
        for p in write_paths {
            sbpl.push_str(&format!("    (subpath \"{}\")\n", p.display()));
        }
        sbpl.push_str(")\n");

        if allow_network {
            sbpl.push_str("(allow network*)\n");
        } else {
            sbpl.push_str("(deny network*)\n");
        }

        sbpl
    }

    pub fn wrap_command(argv: &[String], profile_text: &str) -> Vec<String> {
        vec![
            "sandbox-exec".to_string(),
            "-p".to_string(),
            profile_text.to_string(),
            "--".to_string(),
        ]
        .into_iter()
        .chain(argv.iter().cloned())
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seatbelt_sbpl_profile_generation() {
        let jail = Path::new("/var/tmp/apple_jail");
        let reads = vec![PathBuf::from("/opt/rust")];
        let writes = vec![PathBuf::from("/var/tmp/apple_jail/out")];

        let profile = SeatbeltProfileBuilder::build_sbpl_profile(jail, &reads, &writes, false);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(deny network*)"));
        assert!(profile.contains("/var/tmp/apple_jail"));
        assert!(profile.contains("/opt/rust"));
        assert!(profile.contains("/var/tmp/apple_jail/out"));
    }

    #[test]
    fn test_seatbelt_wrap_command() {
        let profile = "(version 1)";
        let cmd = vec!["clang".to_string(), "-o".to_string(), "main".to_string()];
        let wrapped = SeatbeltProfileBuilder::wrap_command(&cmd, profile);

        assert_eq!(wrapped[0], "sandbox-exec");
        assert_eq!(wrapped[1], "-p");
        assert_eq!(wrapped[2], "(version 1)");
        assert_eq!(wrapped[3], "--");
        assert_eq!(wrapped[4], "clang");
    }
}
