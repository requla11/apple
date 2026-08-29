#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsSecurityConfig {
    pub low_integrity: bool,
    pub strip_admin_privileges: bool,
    pub appcontainer_isolation: bool,
}

impl Default for WindowsSecurityConfig {
    fn default() -> Self {
        Self {
            low_integrity: true,
            strip_admin_privileges: true,
            appcontainer_isolation: false,
        }
    }
}

pub struct WindowsTokenSanitizer;

impl WindowsTokenSanitizer {
    pub fn is_supported() -> bool {
        cfg!(windows)
    }

    pub fn build_security_descriptor_flags(config: &WindowsSecurityConfig) -> u32 {
        let mut flags = 0;
        if config.low_integrity {
            flags |= 0x0001;
        }
        if config.strip_admin_privileges {
            flags |= 0x0002;
        }
        if config.appcontainer_isolation {
            flags |= 0x0004;
        }
        flags
    }
}

pub struct AppContainerProfileManager;

impl AppContainerProfileManager {
    pub fn generate_container_name(task_id: &str) -> String {
        format!("AppleSandbox.Task.{}", task_id.replace('-', "."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_security_config_defaults() {
        let cfg = WindowsSecurityConfig::default();
        assert!(cfg.low_integrity);
        assert!(cfg.strip_admin_privileges);
        assert!(!cfg.appcontainer_isolation);
    }

    #[test]
    fn test_appcontainer_name_generation() {
        let name = AppContainerProfileManager::generate_container_name("build-task-123");
        assert_eq!(name, "AppleSandbox.Task.build.task.123");
    }

    #[test]
    fn test_security_descriptor_flags() {
        let mut cfg = WindowsSecurityConfig::default();
        let flags1 = WindowsTokenSanitizer::build_security_descriptor_flags(&cfg);
        assert_eq!(flags1, 0x0001 | 0x0002);

        cfg.appcontainer_isolation = true;
        let flags2 = WindowsTokenSanitizer::build_security_descriptor_flags(&cfg);
        assert_eq!(flags2, 0x0001 | 0x0002 | 0x0004);
    }
}
