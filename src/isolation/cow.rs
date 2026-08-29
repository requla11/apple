use std::path::Path;

pub struct CowCloner;

impl CowCloner {
    pub fn clone_file(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }

        #[cfg(target_os = "macos")]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            let src_cstr = CString::new(src.as_os_str().as_bytes())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            let dst_cstr = CString::new(dst.as_os_str().as_bytes())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

            let res = unsafe { libc::clonefile(src_cstr.as_ptr(), dst_cstr.as_ptr(), 0) };
            if res == 0 {
                return Ok(());
            }
        }

        if std::fs::hard_link(src, dst).is_ok() {
            return Ok(());
        }

        std::fs::copy(src, dst).map(|_| ())
    }

    pub fn clone_dir_tree(src: &Path, dst: &Path) -> Result<u64, std::io::Error> {
        if !src.exists() {
            return Ok(0);
        }
        if src.is_file() {
            Self::clone_file(src, dst)?;
            return Ok(1);
        }

        std::fs::create_dir_all(dst)?;
        let mut count = 0;

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if entry_path.is_dir() {
                count += Self::clone_dir_tree(&entry_path, &dest_path)?;
            } else {
                Self::clone_file(&entry_path, &dest_path)?;
                count += 1;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cow_cloner_file_and_tree() {
        let temp = TempDir::new().unwrap();
        let src_dir = temp.path().join("src");
        let dst_dir = temp.path().join("dst");

        std::fs::create_dir_all(src_dir.join("sub")).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"file a").unwrap();
        std::fs::write(src_dir.join("sub").join("b.txt"), b"file b").unwrap();

        let count = CowCloner::clone_dir_tree(&src_dir, &dst_dir).unwrap();
        assert_eq!(count, 2);

        assert!(dst_dir.join("a.txt").exists());
        assert!(dst_dir.join("sub").join("b.txt").exists());
        assert_eq!(std::fs::read(dst_dir.join("a.txt")).unwrap(), b"file a");
    }
}
