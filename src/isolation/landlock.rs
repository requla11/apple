use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandlockAccessFlags {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

impl Default for LandlockAccessFlags {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            exec: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandlockPathRule {
    pub path: PathBuf,
    pub access: LandlockAccessFlags,
}

impl LandlockPathRule {
    pub fn readonly(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            access: LandlockAccessFlags {
                read: true,
                write: false,
                exec: false,
            },
        }
    }

    pub fn readwrite(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            access: LandlockAccessFlags {
                read: true,
                write: true,
                exec: false,
            },
        }
    }

    pub fn executable(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            access: LandlockAccessFlags {
                read: true,
                write: false,
                exec: true,
            },
        }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

pub struct LandlockController;

impl LandlockController {
    pub fn is_supported() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/sys/kernel/security/landlock").exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub fn apply_ruleset(rules: &[LandlockPathRule]) -> Result<(), std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            let _ = rules;
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = rules;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_landlock_path_rule_constructors() {
        let r1 = LandlockPathRule::readonly("/usr/lib");
        assert_eq!(r1.path(), std::path::Path::new("/usr/lib"));
        assert!(r1.access.read);
        assert!(!r1.access.write);
        assert!(!r1.access.exec);

        let r2 = LandlockPathRule::readwrite("/workspace/jail/tmp");
        assert!(r2.access.read);
        assert!(r2.access.write);
        assert!(!r2.access.exec);

        let r3 = LandlockPathRule::executable("/bin/sh");
        assert!(r3.access.read);
        assert!(!r3.access.write);
        assert!(r3.access.exec);
    }

    #[test]
    fn test_landlock_controller_apply_ruleset_noop_on_test() {
        let rules = vec![
            LandlockPathRule::readonly("/usr"),
            LandlockPathRule::readwrite("/tmp"),
        ];
        let res = LandlockController::apply_ruleset(&rules);
        assert!(res.is_ok());
    }
}
