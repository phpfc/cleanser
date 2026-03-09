import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { ScanConfig, ScanResults, ScanProgress } from "../types";

export function useScan() {
  const [results, setResults] = useState<ScanResults | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Listen to scan progress events
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<ScanProgress>("scan:progress", (event) => {
      setProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Load cached scan on mount
  useEffect(() => {
    invoke<ScanResults | null>("get_cached_scan")
      .then((cached) => {
        if (cached) setResults(cached);
      })
      .catch(console.error);
  }, []);

  const scan = useCallback(async (config: ScanConfig) => {
    setIsScanning(true);
    setError(null);
    setProgress(null);

    try {
      const scanResults = await invoke<ScanResults>("scan", { config });
      setResults(scanResults);
      return scanResults;
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      setError(errorMsg);
      throw e;
    } finally {
      setIsScanning(false);
      setProgress(null);
    }
  }, []);

  const clearResults = useCallback(() => {
    setResults(null);
    setError(null);
  }, []);

  return {
    results,
    isScanning,
    progress,
    error,
    scan,
    clearResults,
  };
}
