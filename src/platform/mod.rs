pub mod detection;
pub mod paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
}

impl Platform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "macos" => Platform::MacOS,
            "linux" => Platform::Linux,
            "windows" => Platform::Windows,
            _ => {
                // Fallback to best guess based on path separators
                if std::path::MAIN_SEPARATOR == '\\' {
                    Platform::Windows
                } else {
                    Platform::Linux
                }
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Platform::MacOS => "macOS",
            Platform::Linux => "Linux",
            Platform::Windows => "Windows",
        }
    }

    pub fn cache_dir_names(&self) -> Vec<&'static str> {
        match self {
            Platform::MacOS => vec!["Caches", "Cache", ".cache"],
            Platform::Linux => vec![".cache", "cache", "Cache"],
            Platform::Windows => vec!["Cache", "Temp", "cache"],
        }
    }

    pub fn log_dir_names(&self) -> Vec<&'static str> {
        match self {
            Platform::MacOS => vec!["Logs", "logs", ".logs"],
            Platform::Linux => vec!["log", ".log", "logs"],
            Platform::Windows => vec!["Logs", "Log", "logs"],
        }
    }

    pub fn temp_extensions(&self) -> Vec<&'static str> {
        match self {
            Platform::MacOS => vec!["tmp", "temp", "cache"],
            Platform::Linux => vec!["tmp", "temp", "swp", "swo"],
            Platform::Windows => vec!["tmp", "temp", "bak"],
        }
    }

    /// Get platform-specific directories that should be skipped during scanning
    pub fn system_protected_dirs(&self) -> Vec<&'static str> {
        match self {
            Platform::MacOS => vec![
                "/System",
                "/Library",
                "/private",
                "/dev",
                "/proc",
                "/cores",
            ],
            Platform::Linux => vec![
                "/sys",
                "/proc",
                "/dev",
                "/boot",
                "/root",
                "/run",
            ],
            Platform::Windows => vec![
                "C:\\Windows",
                "C:\\Program Files",
                "C:\\Program Files (x86)",
            ],
        }
    }

    /// Get common package manager cache directories
    pub fn package_manager_caches(&self) -> Vec<&'static str> {
        match self {
            Platform::MacOS => vec![
                "Library/Caches/Homebrew",
                ".cargo/registry",
                ".npm/_cacache",
                ".m2/repository",
            ],
            Platform::Linux => vec![
                ".cache/pip",
                ".cargo/registry",
                ".npm/_cacache",
                ".m2/repository",
                ".cache/yarn",
            ],
            Platform::Windows => vec![
                "AppData\\Local\\pip\\Cache",
                ".cargo\\registry",
                "AppData\\Roaming\\npm-cache",
            ],
        }
    }
}

/// Common directories to check across all platforms
pub fn common_cache_patterns() -> Vec<&'static str> {
    vec![
        "node_modules/.cache",
        ".gradle/caches",
        ".cache",
        "cache",
        "tmp",
        "temp",
    ]
}

/// Common build artifact patterns across all platforms
pub fn common_build_patterns() -> Vec<&'static str> {
    vec![
        "target",
        "build",
        "dist",
        "out",
        ".next",
        ".nuxt",
        "__pycache__",
        "node_modules",
    ]
}
