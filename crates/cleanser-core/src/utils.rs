//! Shared utility functions.

use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

/// Calculate the total size of a directory.
///
/// Uses parallel iteration for better performance on large directories.
/// Does not follow symlinks.
///
/// # Arguments
///
/// * `path` - The path to the directory
///
/// # Returns
///
/// The total size in bytes of all files in the directory tree.
pub fn get_dir_size(path: &Path) -> Result<u64> {
    let total: u64 = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_get_dir_size_empty() {
        let dir = tempdir().unwrap();
        let size = get_dir_size(dir.path()).unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn test_get_dir_size_with_files() {
        let dir = tempdir().unwrap();

        // Create a file with known content
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap(); // 13 bytes

        let size = get_dir_size(dir.path()).unwrap();
        assert_eq!(size, 13);
    }

    #[test]
    fn test_get_dir_size_nested() {
        let dir = tempdir().unwrap();

        // Create nested structure
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let file1 = dir.path().join("file1.txt");
        let file2 = subdir.join("file2.txt");

        let mut f1 = File::create(&file1).unwrap();
        f1.write_all(b"12345").unwrap(); // 5 bytes

        let mut f2 = File::create(&file2).unwrap();
        f2.write_all(b"1234567890").unwrap(); // 10 bytes

        let size = get_dir_size(dir.path()).unwrap();
        assert_eq!(size, 15);
    }
}
