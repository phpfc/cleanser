import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { MapStats, MapProgress } from "../types";
import { useI18n } from "../i18n";

export function MapView() {
  const { t } = useI18n();
  const [stats, setStats] = useState<MapStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRebuilding, setIsRebuilding] = useState(false);
  const [progress, setProgress] = useState<MapProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadStats();

    let unlisten: UnlistenFn | null = null;
    listen<MapProgress>("map:progress", (event) => {
      setProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const loadStats = async () => {
    setIsLoading(true);
    try {
      const mapStats = await invoke<MapStats | null>("get_map_stats");
      setStats(mapStats);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const handleRebuild = async () => {
    setIsRebuilding(true);
    setProgress(null);
    setError(null);
    try {
      const newStats = await invoke<MapStats>("rebuild_map");
      setStats(newStats);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsRebuilding(false);
      setProgress(null);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted">{t("loading")}</div>
      </div>
    );
  }

  const maxTagCount = stats?.tags[0]?.[1] || 1;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h2 className="text-xl font-bold text-primary">{t("filesystemMap")}</h2>
          <p className="text-sm text-secondary">
            {t("intelligentMapping")}
          </p>
        </div>

        <button
          onClick={handleRebuild}
          disabled={isRebuilding}
          className="btn btn-primary"
        >
          {isRebuilding ? (
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none"/>
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
              </svg>
              {t("rebuilding")}
            </span>
          ) : (
            <>
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
              </svg>
              {t("rebuildMap")}
            </>
          )}
        </button>
      </div>

      {/* Progress during rebuild */}
      {isRebuilding && progress && (
        <div className="card p-4 mb-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-semibold accent-text">{t("mapping")}</span>
            {progress.total > 0 && (
              <span className="text-sm text-secondary font-mono">
                {progress.current}/{progress.total}
              </span>
            )}
          </div>
          <div className="progress-bar h-2 mb-2">
            <div
              className="progress-bar-fill h-full"
              style={{
                width: progress.total > 0 ? `${(progress.current / progress.total) * 100}%` : "100%",
                animation: progress.total === 0 ? "pulse-orange 1.5s ease-in-out infinite" : "none",
              }}
            />
          </div>
          <p className="text-xs text-muted truncate font-mono">
            {progress.message}
          </p>
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="card p-4 mb-4 category-card-risky bg-risky/5">
          <p className="risk-risky">{error}</p>
        </div>
      )}

      {/* No map */}
      {!stats && !isRebuilding && (
        <div className="flex-1 flex flex-col items-center justify-center">
          <svg className="w-16 h-16 text-muted mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7"/>
          </svg>
          <p className="text-lg font-semibold text-primary mb-2">
            {t("noMapFound")}
          </p>
          <p className="text-muted mb-4">
            {t("createMapDescription")}
          </p>
          <button onClick={handleRebuild} className="btn btn-primary">
            {t("createMap")}
          </button>
        </div>
      )}

      {/* Stats */}
      {stats && !isRebuilding && (
        <div className="flex-1 overflow-y-auto">
          {/* Summary cards */}
          <div className="grid grid-cols-3 gap-4 mb-6">
            <div className="card p-4">
              <p className="text-sm text-muted mb-1">{t("totalMapped")}</p>
              <p className="text-2xl font-bold text-primary">{stats.total_directories.toLocaleString()}</p>
              <p className="text-xs text-muted">{t("directories")}</p>
            </div>
            <div className="card p-4">
              <p className="text-sm text-muted mb-1">{t("cleanable")}</p>
              <p className="text-2xl font-bold risk-safe">{stats.cleanable_count.toLocaleString()}</p>
              <p className="text-xs text-muted">{t("directories")}</p>
            </div>
            <div className="card p-4">
              <p className="text-sm text-muted mb-1">{t("lastScan")}</p>
              <p className="text-lg font-semibold text-primary">{stats.created_at}</p>
              {stats.is_stale && (
                <p className="text-xs risk-moderate">{t("outdated")}</p>
              )}
            </div>
          </div>

          {/* Categories */}
          <div className="card p-4 mb-4">
            <h3 className="text-sm font-semibold text-secondary mb-4 uppercase tracking-wide">
              {t("byCategory")}
            </h3>
            <div className="space-y-3">
              {Object.entries(stats.categories)
                .sort((a, b) => b[1] - a[1])
                .map(([category, count]) => {
                  const maxCount = Math.max(...Object.values(stats.categories));
                  const percentage = (count / maxCount) * 100;
                  const isCleanable = category === "Cache/Temp" || category === "Build Artifacts";

                  return (
                    <div key={category} className="flex items-center gap-3">
                      <span className={`w-32 text-sm ${isCleanable ? "risk-safe" : "text-primary"}`}>
                        {category}
                      </span>
                      <div className="flex-1 progress-bar h-3">
                        <div
                          className={`h-full rounded-full transition-all ${isCleanable ? "bg-safe" : "bg-[var(--orange-primary)]"}`}
                          style={{ width: `${percentage}%` }}
                        />
                      </div>
                      <span className="text-sm text-muted w-16 text-right font-mono">
                        {count.toLocaleString()}
                      </span>
                    </div>
                  );
                })}
            </div>
          </div>

          {/* Tags */}
          <div className="card p-4">
            <h3 className="text-sm font-semibold text-secondary mb-4 uppercase tracking-wide">
              {t("topTypes")}
            </h3>
            <div className="space-y-2">
              {stats.tags.map(([tag, count]) => {
                const percentage = (count / maxTagCount) * 100;
                return (
                  <div key={tag} className="flex items-center gap-3">
                    <span className="w-28 text-sm accent-text truncate" title={tag}>
                      {tag}
                    </span>
                    <div className="flex-1 progress-bar h-2">
                      <div
                        className="progress-bar-fill h-full"
                        style={{ width: `${percentage}%` }}
                      />
                    </div>
                    <span className="text-sm text-muted w-12 text-right font-mono">
                      {count}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
