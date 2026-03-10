import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { VersionInfo } from "../types";

export function useVersion() {
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const checkVersion = async () => {
      try {
        setLoading(true);
        const info = await invoke<VersionInfo>("check_version");
        setVersionInfo(info);
        setError(null);
      } catch (e) {
        // Don't show error for version check failures - it's not critical
        console.warn("Version check failed:", e);
        // Still try to get the current version
        try {
          const current = await invoke<string>("get_current_version");
          setVersionInfo({
            current,
            latest: null,
            update_available: false,
            release_url: null,
          });
        } catch {
          setError(String(e));
        }
      } finally {
        setLoading(false);
      }
    };

    checkVersion();
  }, []);

  return { versionInfo, loading, error };
}
