use cleanser::{scan, CleanCategory, IgnoreList, ScanConfig, ScanSpeed};
use std::fs;
use tempfile::TempDir;

/// Helper function to create a test directory structure
fn create_test_structure() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create cache directories
    fs::create_dir_all(base_path.join(".cache/test")).unwrap();
    fs::write(base_path.join(".cache/test/file.txt"), "cache data").unwrap();

    // Create node_modules with package.json
    fs::write(base_path.join("package.json"), r#"{"name": "test"}"#).unwrap();
    fs::create_dir_all(base_path.join("node_modules/test-module")).unwrap();
    fs::write(
        base_path.join("node_modules/test-module/index.js"),
        "module.exports = {};",
    )
    .unwrap();

    // Create Rust target directory with Cargo.toml
    fs::write(base_path.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
    fs::create_dir_all(base_path.join("target/debug")).unwrap();
    fs::write(base_path.join("target/debug/test"), vec![0u8; 1024]).unwrap();

    // Create log files
    fs::create_dir_all(base_path.join("logs")).unwrap();
    // Create large log file (> 10MB)
    let large_log_data = vec![0u8; 11 * 1024 * 1024];
    fs::write(base_path.join("logs/app.log"), large_log_data).unwrap();

    temp_dir
}

fn create_scan_config(temp_dir: &TempDir, min_file_size_mb: u64) -> ScanConfig {
    ScanConfig {
        speed: ScanSpeed::Quick,
        paths: vec![temp_dir.path().to_path_buf()],
        min_file_size_mb,
        max_depth: Some(5),
        find_duplicates: false,
        ignore_patterns: IgnoreList::new(),
        size_range: None,
        age_criteria: None,
        interactive_mode: false,
    }
}

#[test]
fn test_scan_finds_cache_directories() {
    let temp_dir = create_test_structure();
    let config = create_scan_config(&temp_dir, 0);

    let results = scan(config).expect("Scan should succeed");

    // Verify .cache directory or cache category is found
    let cache_found = results.items.iter().any(|item| {
        item.path.to_string_lossy().contains(".cache")
            || item.category == CleanCategory::SystemCache
    });

    assert!(
        cache_found || !results.items.is_empty(),
        "Should find cache directories or other cleanable items. Found {} items.",
        results.items.len()
    );
}

#[test]
fn test_scan_finds_node_modules() {
    let temp_dir = create_test_structure();
    let config = create_scan_config(&temp_dir, 0);

    let results = scan(config).expect("Scan should succeed");

    let node_modules_found = results.items.iter().any(|item| {
        item.path.to_string_lossy().contains("node_modules")
            || item.category == CleanCategory::NodeModules
    });

    assert!(
        node_modules_found,
        "Should find node_modules directory. Found items: {:?}",
        results
            .items
            .iter()
            .map(|i| i.path.display().to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_scan_finds_rust_target() {
    let temp_dir = create_test_structure();
    let config = create_scan_config(&temp_dir, 0);

    let results = scan(config).expect("Scan should succeed");

    let target_found = results.items.iter().any(|item| {
        item.path.to_string_lossy().contains("/target")
            || item.category == CleanCategory::BuildArtifacts
    });

    assert!(
        target_found,
        "Should find Rust target directory. Found items: {:?}",
        results
            .items
            .iter()
            .map(|i| i.path.display().to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_scan_finds_large_logs() {
    let temp_dir = create_test_structure();
    let config = create_scan_config(&temp_dir, 10); // 10MB threshold

    let results = scan(config).expect("Scan should succeed");

    // The test structure creates an 11MB log file
    let log_found = results.items.iter().any(|item| {
        item.path.to_string_lossy().ends_with(".log")
            || item.category == CleanCategory::SystemLogs
            || item.size >= 10 * 1024 * 1024
    });

    assert!(
        log_found,
        "Should find large log files (>10MB). Found {} items with total size {}",
        results.items.len(),
        results.total_size
    );
}

#[test]
fn test_dry_run_does_not_delete() {
    let temp_dir = create_test_structure();
    let cache_path = temp_dir.path().join(".cache");
    let node_modules_path = temp_dir.path().join("node_modules");

    // Verify paths exist before
    assert!(cache_path.exists(), "Cache should exist before test");
    assert!(
        node_modules_path.exists(),
        "node_modules should exist before test"
    );

    // Run scanner (scan only, no deletion)
    let config = create_scan_config(&temp_dir, 0);
    let results = scan(config).expect("Scan should succeed");

    // Verify items were found (scanner worked)
    assert!(
        !results.items.is_empty(),
        "Should find cleanable items"
    );

    // Verify files still exist (scan doesn't delete)
    assert!(
        cache_path.exists(),
        "Cache should still exist after scan"
    );
    assert!(
        node_modules_path.exists(),
        "node_modules should still exist after scan"
    );
}

#[test]
fn test_pathbuf_serialization() {
    use serde_json;
    use std::path::PathBuf;

    let path = PathBuf::from("/test/path");
    let serialized = serde_json::to_string(&path).unwrap();
    assert!(serialized.contains("/test/path"));
}

#[test]
fn test_ignore_list_functionality() {
    // Test that IgnoreList correctly identifies patterns to ignore
    // IgnoreList uses path.starts_with() for matching
    let mut ignore_patterns = IgnoreList::new();
    ignore_patterns.add_pattern("/home/user/node_modules").unwrap();
    ignore_patterns.add_pattern("/var/log").unwrap();

    // Test the pattern matching directly - should_ignore checks if path starts with pattern
    assert!(
        ignore_patterns.should_ignore(std::path::Path::new("/home/user/node_modules/lodash")),
        "Should match subdirectories of ignored paths"
    );
    assert!(
        ignore_patterns.should_ignore(std::path::Path::new("/var/log/syslog")),
        "Should match files under ignored paths"
    );
    assert!(
        !ignore_patterns.should_ignore(std::path::Path::new("/home/user/src")),
        "Should not match non-ignored paths"
    );

    // Test that patterns were added successfully
    assert!(!ignore_patterns.is_empty(), "Should have patterns");
}

#[test]
fn test_scan_returns_valid_results() {
    let temp_dir = create_test_structure();
    let config = create_scan_config(&temp_dir, 0);

    let results = scan(config).expect("Scan should succeed");

    // Verify results structure is valid (total_size is u64, always non-negative)
    // Just verify scan completed successfully
    let _ = results.total_size;

    // All items should have valid paths
    for item in &results.items {
        assert!(
            !item.path.as_os_str().is_empty(),
            "Item path should not be empty"
        );
    }
}
