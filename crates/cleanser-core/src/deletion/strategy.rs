//! Deletion strategy types and traits.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Method for deleting files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeletionMethod {
    /// Standard filesystem deletion (default)
    #[default]
    Standard,
    /// Move to system trash for recovery
    Trash,
    /// Secure deletion with data overwrite
    Secure(SecureDeleteConfig),
}

/// Configuration for secure deletion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecureDeleteConfig {
    /// Number of overwrite passes
    pub passes: u8,
    /// Pattern to use for overwriting
    pub pattern: SecureDeletePattern,
}

impl Default for SecureDeleteConfig {
    fn default() -> Self {
        Self {
            passes: 3,
            pattern: SecureDeletePattern::DoD522022M,
        }
    }
}

impl SecureDeleteConfig {
    /// Create config with specified number of passes using DoD pattern
    pub fn with_passes(passes: u8) -> Self {
        Self {
            passes,
            pattern: SecureDeletePattern::DoD522022M,
        }
    }

    /// Create config for single-pass zero overwrite
    pub fn zeros() -> Self {
        Self {
            passes: 1,
            pattern: SecureDeletePattern::Zeros,
        }
    }

    /// Create config for single-pass random overwrite
    pub fn random() -> Self {
        Self {
            passes: 1,
            pattern: SecureDeletePattern::Random,
        }
    }

    /// Create config for DoD 5220.22-M standard (3 passes)
    pub fn dod() -> Self {
        Self::default()
    }

    /// Create config for Gutmann method (35 passes)
    pub fn gutmann() -> Self {
        Self {
            passes: 35,
            pattern: SecureDeletePattern::Gutmann,
        }
    }
}

/// Pattern used for secure overwriting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecureDeletePattern {
    /// Single pass with zeros (0x00)
    Zeros,
    /// Single pass with random data
    Random,
    /// DoD 5220.22-M standard: pass 1 = 0x00, pass 2 = 0xFF, pass 3 = random
    #[default]
    DoD522022M,
    /// Gutmann method with 35 specific patterns
    Gutmann,
}

impl SecureDeletePattern {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Zeros => "Single pass with zeros",
            Self::Random => "Single pass with random data",
            Self::DoD522022M => "DoD 5220.22-M (3 passes: zeros, ones, random)",
            Self::Gutmann => "Gutmann method (35 passes)",
        }
    }
}

/// Progress callback for deletion operations
pub trait DeletionProgress: Send + Sync {
    /// Called when starting to process a file
    fn on_file_start(&self, path: &Path, size: u64);

    /// Called when starting an overwrite pass
    fn on_pass_start(&self, pass: u8, total_passes: u8);

    /// Called periodically during overwrite with bytes written
    fn on_bytes_written(&self, bytes_written: u64, total_bytes: u64);

    /// Called when a file is completely processed
    fn on_file_complete(&self, path: &Path);
}

/// No-op implementation of DeletionProgress
pub struct NoOpDeletionProgress;

impl DeletionProgress for NoOpDeletionProgress {
    fn on_file_start(&self, _path: &Path, _size: u64) {}
    fn on_pass_start(&self, _pass: u8, _total_passes: u8) {}
    fn on_bytes_written(&self, _bytes_written: u64, _total_bytes: u64) {}
    fn on_file_complete(&self, _path: &Path) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_deletion_method() {
        assert_eq!(DeletionMethod::default(), DeletionMethod::Standard);
    }

    #[test]
    fn test_secure_delete_config_defaults() {
        let config = SecureDeleteConfig::default();
        assert_eq!(config.passes, 3);
        assert_eq!(config.pattern, SecureDeletePattern::DoD522022M);
    }

    #[test]
    fn test_secure_delete_config_builders() {
        assert_eq!(SecureDeleteConfig::zeros().passes, 1);
        assert_eq!(
            SecureDeleteConfig::zeros().pattern,
            SecureDeletePattern::Zeros
        );

        assert_eq!(SecureDeleteConfig::random().passes, 1);
        assert_eq!(
            SecureDeleteConfig::random().pattern,
            SecureDeletePattern::Random
        );

        assert_eq!(SecureDeleteConfig::dod().passes, 3);
        assert_eq!(SecureDeleteConfig::gutmann().passes, 35);
    }

    #[test]
    fn test_serialization() {
        let method = DeletionMethod::Secure(SecureDeleteConfig::default());
        let json = serde_json::to_string(&method).unwrap();
        assert!(json.contains("secure"));

        let parsed: DeletionMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, method);
    }
}
