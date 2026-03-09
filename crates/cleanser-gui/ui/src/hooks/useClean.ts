import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { CleanResult, CleanProgress } from "../types";

export function useClean() {
  const [isCleaning, setIsCleaning] = useState(false);
  const [progress, setProgress] = useState<CleanProgress | null>(null);
  const [result, setResult] = useState<CleanResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Listen to clean progress events
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    listen<CleanProgress>("clean:progress", (event) => {
      setProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const cleanItems = useCallback(
    async (paths: string[], dryRun: boolean = false) => {
      setIsCleaning(true);
      setError(null);
      setProgress(null);
      setResult(null);

      try {
        const cleanResult = await invoke<CleanResult>("clean_items", {
          paths,
          dryRun,
        });
        setResult(cleanResult);
        return cleanResult;
      } catch (e) {
        const errorMsg = e instanceof Error ? e.message : String(e);
        setError(errorMsg);
        throw e;
      } finally {
        setIsCleaning(false);
        setProgress(null);
      }
    },
    []
  );

  const clearResult = useCallback(() => {
    setResult(null);
    setError(null);
  }, []);

  return {
    isCleaning,
    progress,
    result,
    error,
    cleanItems,
    clearResult,
  };
}
