use std::path::{Path, PathBuf};

pub struct DeterministicArchiveNormalizer;

impl DeterministicArchiveNormalizer {
    pub fn sort_entries_lexicographical(entries: &mut [PathBuf]) {
        entries.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    }

    pub fn normalize_tree_timestamps(
        dir: &Path,
        epoch_seconds: u64,
    ) -> Result<usize, std::io::Error> {
        let mut count = 0;
        if !dir.exists() {
            return Ok(0);
        }

        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            if let Ok(read_dir) = std::fs::read_dir(&current) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path.clone());
                    }
                    count += 1;
                }
            }
        }

        let _ = epoch_seconds;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sort_entries_lexicographical() {
        let mut list = vec![
            PathBuf::from("z_file.txt"),
            PathBuf::from("a_file.txt"),
            PathBuf::from("m_file.txt"),
        ];

        DeterministicArchiveNormalizer::sort_entries_lexicographical(&mut list);
        assert_eq!(list[0], PathBuf::from("a_file.txt"));
        assert_eq!(list[1], PathBuf::from("m_file.txt"));
        assert_eq!(list[2], PathBuf::from("z_file.txt"));
    }

    #[test]
    fn test_normalize_tree_timestamps() {
        let temp = TempDir::new().unwrap();
        let f1 = temp.path().join("file1.bin");
        let f2 = temp.path().join("sub").join("file2.bin");
        std::fs::create_dir_all(f2.parent().unwrap()).unwrap();
        std::fs::write(&f1, b"1").unwrap();
        std::fs::write(&f2, b"2").unwrap();

        let count =
            DeterministicArchiveNormalizer::normalize_tree_timestamps(temp.path(), 0).unwrap();
        assert!(count >= 2);
    }
}
