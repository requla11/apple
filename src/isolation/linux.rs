use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxIsolationTier {
    FullNamespacesAndCgroup,
    LandlockAndSeccomp,
    SeccompOnly,
    FallbackJail,
}

pub struct LinuxCapabilityProber;

impl LinuxCapabilityProber {
    pub fn probe_tier() -> LinuxIsolationTier {
        #[cfg(target_os = "linux")]
        {
            let userns_enabled =
                std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
                    .map(|s| s.trim() == "1")
                    .unwrap_or(true);

            let landlock_available = Path::new("/sys/kernel/security/lsm").exists()
                || Path::new("/proc/sys/kernel").exists();

            if userns_enabled {
                LinuxIsolationTier::FullNamespacesAndCgroup
            } else if landlock_available {
                LinuxIsolationTier::LandlockAndSeccomp
            } else {
                LinuxIsolationTier::SeccompOnly
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            LinuxIsolationTier::FallbackJail
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxNamespaceConfig {
    pub new_net: bool,
    pub new_pid: bool,
    pub new_mount: bool,
    pub new_ipc: bool,
    pub new_uts: bool,
    pub new_user: bool,
}

impl Default for LinuxNamespaceConfig {
    fn default() -> Self {
        Self {
            new_net: true,
            new_pid: true,
            new_mount: true,
            new_ipc: true,
            new_uts: true,
            new_user: true,
        }
    }
}

impl LinuxNamespaceConfig {
    #[allow(unused_mut)]
    pub fn compute_clone_flags(&self) -> i32 {
        let mut flags = 0;
        #[cfg(target_os = "linux")]
        {
            if self.new_net {
                flags |= libc::CLONE_NEWNET;
            }
            if self.new_pid {
                flags |= libc::CLONE_NEWPID;
            }
            if self.new_mount {
                flags |= libc::CLONE_NEWNS;
            }
            if self.new_ipc {
                flags |= libc::CLONE_NEWIPC;
            }
            if self.new_uts {
                flags |= libc::CLONE_NEWUTS;
            }
            if self.new_user {
                flags |= libc::CLONE_NEWUSER;
            }
        }
        flags
    }
}

#[derive(Debug, Clone)]
pub struct CgroupV2Controller {
    cgroup_root: PathBuf,
    task_id: String,
}

impl CgroupV2Controller {
    pub fn new(cgroup_base: impl AsRef<Path>, task_id: impl Into<String>) -> Self {
        let task_id = task_id.into();
        let cgroup_root = cgroup_base.as_ref().join("apple_sandbox").join(&task_id);
        Self {
            cgroup_root,
            task_id,
        }
    }

    pub fn cgroup_root(&self) -> &Path {
        &self.cgroup_root
    }

    pub fn setup_limits(
        &self,
        memory_max_bytes: Option<u64>,
        cpu_max_quota_us: Option<u64>,
        cpu_max_period_us: u64,
        cpuset: Option<&str>,
    ) -> Result<(), std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            std::fs::create_dir_all(&self.cgroup_root)?;

            if let Some(mem) = memory_max_bytes {
                std::fs::write(self.cgroup_root.join("memory.max"), mem.to_string())?;
            }

            if let Some(quota) = cpu_max_quota_us {
                std::fs::write(
                    self.cgroup_root.join("cpu.max"),
                    format!("{quota} {cpu_max_period_us}"),
                )?;
            }

            if let Some(cores) = cpuset {
                std::fs::write(self.cgroup_root.join("cpuset.cpus"), cores)?;
            }
        }
        let _ = (
            memory_max_bytes,
            cpu_max_quota_us,
            cpu_max_period_us,
            cpuset,
        );
        Ok(())
    }

    pub fn attach_pid(&self, pid: u32) -> Result<(), std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            if self.cgroup_root.exists() {
                std::fs::write(self.cgroup_root.join("cgroup.procs"), pid.to_string())?;
            }
        }
        let _ = pid;
        Ok(())
    }

    pub fn cleanup(&self) -> Result<(), std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            if self.cgroup_root.exists() {
                std::fs::remove_dir(&self.cgroup_root)?;
            }
        }
        Ok(())
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

pub struct SeccompProfileBuilder;

impl SeccompProfileBuilder {
    pub fn build_strict_filter(allow_network: bool) -> Vec<&'static str> {
        let mut denied = vec![
            "ptrace",
            "process_vm_readv",
            "process_vm_writev",
            "sys_chroot",
            "reboot",
            "kexec_load",
            "init_module",
            "finit_module",
            "delete_module",
            "iopl",
            "ioperm",
            "swapon",
            "swapoff",
        ];

        if !allow_network {
            denied.extend_from_slice(&[
                "socket", "connect", "bind", "listen", "accept", "accept4", "sendto", "recvfrom",
                "sendmsg", "recvmsg",
            ]);
        }

        denied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_capability_prober() {
        let tier = LinuxCapabilityProber::probe_tier();
        assert!(matches!(
            tier,
            LinuxIsolationTier::FullNamespacesAndCgroup
                | LinuxIsolationTier::LandlockAndSeccomp
                | LinuxIsolationTier::SeccompOnly
                | LinuxIsolationTier::FallbackJail
        ));
    }

    #[test]
    fn test_linux_namespace_config_defaults() {
        let cfg = LinuxNamespaceConfig::default();
        assert!(cfg.new_net);
        assert!(cfg.new_pid);
        assert!(cfg.new_mount);
        assert!(cfg.new_ipc);
        assert!(cfg.new_uts);
        assert!(cfg.new_user);
    }

    #[test]
    fn test_cgroupv2_controller_path() {
        let ctrl = CgroupV2Controller::new("/sys/fs/cgroup", "task_build_1");
        assert_eq!(ctrl.task_id(), "task_build_1");
        assert!(ctrl.cgroup_root().ends_with("apple_sandbox/task_build_1"));
    }

    #[test]
    fn test_seccomp_profile_filter_offline_denials() {
        let offline_filter = SeccompProfileBuilder::build_strict_filter(false);
        assert!(offline_filter.contains(&"ptrace"));
        assert!(offline_filter.contains(&"socket"));
        assert!(offline_filter.contains(&"connect"));

        let online_filter = SeccompProfileBuilder::build_strict_filter(true);
        assert!(online_filter.contains(&"ptrace"));
        assert!(!online_filter.contains(&"socket"));
    }
}
