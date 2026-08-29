use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualProjectionPlanner {
    excluded_patterns: Vec<String>,
}

impl Default for VirtualProjectionPlanner {
    fn default() -> Self {
        Self {
            excluded_patterns: vec![
                ".git".to_string(),
                ".svn".to_string(),
                ".hg".to_string(),
                ".apple-scratch".to_string(),
                "node_modules/.cache".to_string(),
                "target".to_string(),
            ],
        }
    }
}

impl VirtualProjectionPlanner {
    pub fn new(custom_exclusions: Vec<String>) -> Self {
        Self {
            excluded_patterns: custom_exclusions,
        }
    }

    pub fn is_excluded(&self, path: &Path, base_dir: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(base_dir) else {
            return false;
        };

        for component in relative.components() {
            let comp_str = component.as_os_str().to_string_lossy();
            for pattern in &self.excluded_patterns {
                if comp_str == *pattern {
                    return true;
                }
            }
        }

        false
    }

    pub fn plan_projection_paths(
        &self,
        source_dir: &Path,
        declared_inputs: &[PathBuf],
    ) -> Vec<PathBuf> {
        if !declared_inputs.is_empty() {
            let mut set = HashSet::new();
            for input in declared_inputs {
                if input.exists() {
                    set.insert(input.clone());
                }
            }
            return set.into_iter().collect();
        }

        let mut projected = Vec::new();
        if let Ok(entries) = std::fs::read_dir(source_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !self.is_excluded(&p, source_dir) {
                    projected.push(p);
                }
            }
        }

        projected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_virtual_projection_planner_exclusions() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join(".git");
        let src_dir = temp.path().join("src");
        let file = temp.path().join("src").join("main.rs");

        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(&file, b"fn main() {}").unwrap();

        let planner = VirtualProjectionPlanner::default();
        assert!(planner.is_excluded(&git_dir, temp.path()));
        assert!(!planner.is_excluded(&src_dir, temp.path()));
        assert!(!planner.is_excluded(&file, temp.path()));
    }

    #[test]
    fn test_plan_projection_with_declared_inputs() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("input1.txt");
        let file2 = temp.path().join("input2.txt");
        std::fs::write(&file1, b"a").unwrap();
        std::fs::write(&file2, b"b").unwrap();

        let planner = VirtualProjectionPlanner::default();
        let plan = planner.plan_projection_paths(temp.path(), std::slice::from_ref(&file1));
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0], file1);
    }
}
