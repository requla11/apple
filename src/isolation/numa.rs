use std::collections::HashMap;

pub struct NumaAffinityController;

impl NumaAffinityController {
    pub fn detect_numa_nodes() -> Vec<u32> {
        #[cfg(target_os = "linux")]
        {
            let mut nodes = Vec::new();
            if let Ok(entries) = std::fs::read_dir("/sys/devices/system/node") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(stripped) = name.strip_prefix("node") {
                        if let Ok(id) = stripped.parse::<u32>() {
                            nodes.push(id);
                        }
                    }
                }
            }
            if !nodes.is_empty() {
                nodes.sort_unstable();
                return nodes;
            }
        }

        vec![0]
    }

    pub fn assign_numa_node(node_id: u32, env: &mut HashMap<String, String>) {
        env.insert("NUMA_NODE".to_string(), node_id.to_string());
        env.insert("OMP_NUM_THREADS".to_string(), "4".to_string());
        env.insert("RAYON_NUM_THREADS".to_string(), "4".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_numa_nodes() {
        let nodes = NumaAffinityController::detect_numa_nodes();
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0], 0);
    }

    #[test]
    fn test_assign_numa_node() {
        let mut env = HashMap::new();
        NumaAffinityController::assign_numa_node(1, &mut env);
        assert_eq!(env.get("NUMA_NODE"), Some(&"1".to_string()));
        assert_eq!(env.get("OMP_NUM_THREADS"), Some(&"4".to_string()));
    }
}
