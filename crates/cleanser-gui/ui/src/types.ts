// Types matching the Rust DTOs

export interface ScanConfig {
  speed: "quick" | "normal" | "thorough";
  min_file_size_mb?: number;
  find_duplicates?: boolean;
}

export interface CleanableItem {
  path: string;
  size: number;
  category: string;
  risk_level: "safe" | "moderate" | "risky";
  description: string;
}

export interface ScanResults {
  items: CleanableItem[];
  total_size: number;
  items_count: number;
  filtered_by_size_count: number;
  filtered_by_age_count: number;
}

export interface CleanResult {
  cleaned_count: number;
  failed_count: number;
  cleaned_size: number;
  failures: [string, string][];
}

export interface ScanProgress {
  phase: string;
  message: string;
  current: number | null;
  total: number | null;
}

export interface CleanProgress {
  phase: string;
  message: string;
  current_item: string | null;
  current: number;
  total: number;
  cleaned_size: number;
}

export interface SystemInfo {
  platform: string;
  home_dir: string;
}

// Grouped items by category
export interface CategoryGroup {
  category: string;
  items: CleanableItem[];
  totalSize: number;
  riskLevel: "safe" | "moderate" | "risky";
}

// Filesystem Map types
export interface MapStats {
  total_directories: number;
  cleanable_count: number;
  created_at: string;
  is_stale: boolean;
  categories: Record<string, number>;
  tags: [string, number][];
}

export interface MapProgress {
  message: string;
  current: number;
  total: number;
}

// Version types
export interface VersionInfo {
  current: string;
  latest: string | null;
  update_available: boolean;
  release_url: string | null;
}

// Trash types
export interface TrashEntry {
  id: string;
  original_path: string;
  trash_path: string;
  size: number;
  is_directory: boolean;
  deleted_at: string;
  age: string;
}

export interface TrashStats {
  location: string;
  item_count: number;
  total_size: number;
  directories: number;
  files: number;
}

// Schedule types
export interface ScheduledJob {
  id: string;
  name: string;
  frequency: string;
  risk_level: string;
  enabled: boolean;
  use_trash: boolean;
  secure_delete: boolean;
  last_run: string | null;
}

export interface CreateJobInput {
  name: string;
  frequency: string;
  risk_level: string;
  use_trash: boolean;
  secure_delete: boolean;
  notify: boolean;
}
