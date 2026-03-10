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
    use std::fs;
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

        // Create a file with known content using fs::write (ensures file is closed)
        let file_path = dir.path().join("test.txt");
        let content = b"Hello, World!"; // 13 bytes
        fs::write(&file_path, content).unwrap();

        let size = get_dir_size(dir.path()).unwrap();
        assert_eq!(size, content.len() as u64);
    }

    #[test]
    fn test_get_dir_size_nested() {
        let dir = tempdir().unwrap();

        // Create nested structure
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let content1 = b"12345"; // 5 bytes
        let content2 = b"1234567890"; // 10 bytes

        fs::write(dir.path().join("file1.txt"), content1).unwrap();
        fs::write(subdir.join("file2.txt"), content2).unwrap();

        let size = get_dir_size(dir.path()).unwrap();
        assert_eq!(size, (content1.len() + content2.len()) as u64);
    }
}
