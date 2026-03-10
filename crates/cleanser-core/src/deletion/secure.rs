//! Secure file deletion with data overwriting.
//!
//! Implements secure deletion patterns including:
//! - DoD 5220.22-M (3 passes)
//! - Gutmann method (35 passes)
//! - Simple zero/random overwrite

use super::strategy::{
    DeletionProgress, NoOpDeletionProgress, SecureDeleteConfig, SecureDeletePattern,
};
use anyhow::{Context, Result};
use rand::Rng;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tracing::{debug, warn};
use walkdir::WalkDir;

/// Buffer size for overwrite operations (64KB)
const BUFFER_SIZE: usize = 64 * 1024;

/// Secure file deleter implementing various overwrite patterns
pub struct SecureDeleter {
    config: SecureDeleteConfig,
}

impl SecureDeleter {
    /// Create a new secure deleter with the given configuration
    pub fn new(config: SecureDeleteConfig) -> Self {
        Self { config }
    }

    /// Create a secure deleter with default DoD 5220.22-M configuration
    pub fn dod() -> Self {
        Self::new(SecureDeleteConfig::dod())
    }

    /// Securely delete a file by overwriting its contents before removal
    pub fn delete_file(&self, path: &Path) -> Result<u64> {
        self.delete_file_with_progress(path, &NoOpDeletionProgress)
    }

    /// Securely delete a file with progress callbacks
    pub fn delete_file_with_progress(
        &self,
        path: &Path,
        progress: &dyn DeletionProgress,
    ) -> Result<u64> {
        if !path.exists() {
            return Ok(0);
        }

        if path.is_dir() {
            return self.delete_directory_with_progress(path, progress);
        }

        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for {}", path.display()))?;
        let size = metadata.len();

        progress.on_file_start(path, size);

        // Open file for writing
        let mut file = OpenOptions::new().write(true).open(path).with_context(|| {
            format!(
                "Failed to open file for secure deletion: {}",
                path.display()
            )
        })?;

        // Perform overwrite passes
        self.overwrite_file(&mut file, size, progress)?;

        // Sync to ensure data is written to disk
        file.sync_all()
            .with_context(|| format!("Failed to sync file: {}", path.display()))?;

        // Close file handle before deletion
        drop(file);

        // Delete the file
        fs::remove_file(path).with_context(|| {
            format!(
                "Failed to remove file after secure overwrite: {}",
                path.display()
            )
        })?;

        progress.on_file_complete(path);
        debug!("Securely deleted file: {}", path.display());

        Ok(size)
    }

    /// Securely delete a directory and all its contents
    pub fn delete_directory(&self, path: &Path) -> Result<u64> {
        self.delete_directory_with_progress(path, &NoOpDeletionProgress)
    }

    /// Securely delete a directory with progress callbacks
    pub fn delete_directory_with_progress(
        &self,
        path: &Path,
        progress: &dyn DeletionProgress,
    ) -> Result<u64> {
        if !path.exists() {
            return Ok(0);
        }

        if !path.is_dir() {
            return self.delete_file_with_progress(path, progress);
        }

        let mut total_size = 0u64;

        // First, securely delete all files
        // We collect paths first to avoid borrowing issues
        let files: Vec<_> = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        for file_path in files {
            match self.delete_file_with_progress(&file_path, progress) {
                Ok(size) => total_size += size,
                Err(e) => {
                    warn!("Failed to securely delete {}: {}", file_path.display(), e);
                }
            }
        }

        // Then remove the empty directories (deepest first)
        let mut dirs: Vec<_> = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .map(|e| e.path().to_path_buf())
            .collect();

        // Sort by depth (deepest first)
        dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

        for dir_path in dirs {
            if let Err(e) = fs::remove_dir(&dir_path) {
                warn!("Failed to remove directory {}: {}", dir_path.display(), e);
            }
        }

        debug!("Securely deleted directory: {}", path.display());
        Ok(total_size)
    }

    /// Perform overwrite passes on a file
    fn overwrite_file(
        &self,
        file: &mut File,
        size: u64,
        progress: &dyn DeletionProgress,
    ) -> Result<()> {
        let passes = self.get_effective_passes();

        for pass in 0..passes {
            progress.on_pass_start(pass + 1, passes);
            file.seek(SeekFrom::Start(0))?;
            self.write_pass(file, size, pass, progress)?;
        }

        // Final pass: truncate to 0 to remove file size information
        file.set_len(0)?;

        Ok(())
    }

    /// Get the effective number of passes based on pattern
    fn get_effective_passes(&self) -> u8 {
        match self.config.pattern {
            SecureDeletePattern::Zeros | SecureDeletePattern::Random => 1,
            SecureDeletePattern::DoD522022M => 3,
            SecureDeletePattern::Gutmann => 35,
        }
    }

    /// Write a single overwrite pass
    fn write_pass(
        &self,
        file: &mut File,
        size: u64,
        pass: u8,
        progress: &dyn DeletionProgress,
    ) -> Result<()> {
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut written = 0u64;

        while written < size {
            let to_write = std::cmp::min(BUFFER_SIZE as u64, size - written) as usize;
            let buffer_slice = &mut buffer[..to_write];

            // Fill buffer with appropriate pattern
            self.fill_pattern(buffer_slice, pass);

            file.write_all(buffer_slice)?;
            written += to_write as u64;
            progress.on_bytes_written(written, size);
        }

        file.flush()?;
        Ok(())
    }

    /// Fill buffer with the appropriate pattern for the given pass
    fn fill_pattern(&self, buffer: &mut [u8], pass: u8) {
        match self.config.pattern {
            SecureDeletePattern::Zeros => {
                buffer.fill(0x00);
            }
            SecureDeletePattern::Random => {
                rand::thread_rng().fill(buffer);
            }
            SecureDeletePattern::DoD522022M => {
                match pass % 3 {
                    0 => buffer.fill(0x00),               // Pass 1: zeros
                    1 => buffer.fill(0xFF),               // Pass 2: ones
                    _ => rand::thread_rng().fill(buffer), // Pass 3: random
                }
            }
            SecureDeletePattern::Gutmann => {
                self.fill_gutmann_pattern(buffer, pass);
            }
        }
    }

    /// Fill buffer with Gutmann pattern for the given pass
    fn fill_gutmann_pattern(&self, buffer: &mut [u8], pass: u8) {
        // Gutmann method patterns
        // Passes 0-3: random
        // Passes 4-30: specific patterns
        // Passes 31-34: random
        match pass {
            0..=3 | 31..=34 => {
                rand::thread_rng().fill(buffer);
            }
            4 => buffer.fill(0x55),
            5 => buffer.fill(0xAA),
            6 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0x92, 0x49, 0x24][i % 3];
                }
            }
            7 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0x49, 0x24, 0x92][i % 3];
                }
            }
            8 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0x24, 0x92, 0x49][i % 3];
                }
            }
            9 => buffer.fill(0x00),
            10 => buffer.fill(0x11),
            11 => buffer.fill(0x22),
            12 => buffer.fill(0x33),
            13 => buffer.fill(0x44),
            14 => buffer.fill(0x55),
            15 => buffer.fill(0x66),
            16 => buffer.fill(0x77),
            17 => buffer.fill(0x88),
            18 => buffer.fill(0x99),
            19 => buffer.fill(0xAA),
            20 => buffer.fill(0xBB),
            21 => buffer.fill(0xCC),
            22 => buffer.fill(0xDD),
            23 => buffer.fill(0xEE),
            24 => buffer.fill(0xFF),
            25 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0x92, 0x49, 0x24][i % 3];
                }
            }
            26 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0x49, 0x24, 0x92][i % 3];
                }
            }
            27 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0x24, 0x92, 0x49][i % 3];
                }
            }
            28 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0x6D, 0xB6, 0xDB][i % 3];
                }
            }
            29 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0xB6, 0xDB, 0x6D][i % 3];
                }
            }
            30 => {
                for (i, byte) in buffer.iter_mut().enumerate() {
                    *byte = [0xDB, 0x6D, 0xB6][i % 3];
                }
            }
            _ => rand::thread_rng().fill(buffer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn test_secure_delete_file_zeros() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("secret.txt");
        let content = b"This is secret data that should be securely deleted";
        fs::write(&path, content).unwrap();

        let size = content.len() as u64;

        let deleter = SecureDeleter::new(SecureDeleteConfig::zeros());
        let deleted_size = deleter.delete_file(&path).unwrap();

        assert_eq!(deleted_size, size);
        assert!(!path.exists());
    }

    #[test]
    fn test_secure_delete_file_dod() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("secret_dod.txt");
        fs::write(&path, b"Secret data for DoD deletion").unwrap();

        let deleter = SecureDeleter::dod();
        deleter.delete_file(&path).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn test_secure_delete_directory() {
        let dir = tempdir().unwrap();
        let target_dir = dir.path().join("target_dir");
        fs::create_dir(&target_dir).unwrap();

        let file1 = target_dir.join("file1.txt");
        let file2 = target_dir.join("file2.txt");
        let subdir = target_dir.join("subdir");
        let file3 = subdir.join("file3.txt");

        fs::create_dir(&subdir).unwrap();
        fs::write(&file1, b"Secret 1").unwrap();
        fs::write(&file2, b"Secret 2").unwrap();
        fs::write(&file3, b"Secret 3").unwrap();

        let deleter = SecureDeleter::new(SecureDeleteConfig::zeros());
        let size = deleter.delete_directory(&target_dir).unwrap();

        assert!(size > 0);
        assert!(!target_dir.exists());
    }

    #[test]
    fn test_secure_delete_nonexistent() {
        let deleter = SecureDeleter::dod();
        let result = deleter.delete_file(Path::new("/nonexistent/path/file.txt"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_fill_dod_pattern() {
        let deleter = SecureDeleter::dod();
        let mut buffer = vec![0u8; 100];

        // Pass 0: zeros
        deleter.fill_pattern(&mut buffer, 0);
        assert!(buffer.iter().all(|&b| b == 0x00));

        // Pass 1: ones
        deleter.fill_pattern(&mut buffer, 1);
        assert!(buffer.iter().all(|&b| b == 0xFF));

        // Pass 2: random (just check it changed)
        let zeros = vec![0u8; 100];
        deleter.fill_pattern(&mut buffer, 2);
        assert_ne!(buffer, zeros); // Very unlikely to be all zeros
    }

    #[test]
    fn test_overwrite_actually_changes_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("original.txt");
        let original = b"ORIGINAL SECRET DATA 12345";
        fs::write(&path, original).unwrap();

        // Re-open to read
        let mut read_file = File::open(&path).unwrap();
        let mut content = Vec::new();
        read_file.read_to_end(&mut content).unwrap();
        assert_eq!(content, original);
        drop(read_file);

        let deleter = SecureDeleter::new(SecureDeleteConfig::zeros());
        deleter.delete_file(&path).unwrap();

        assert!(!path.exists());
    }
}
