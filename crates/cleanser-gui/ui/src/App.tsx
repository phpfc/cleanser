import { useState, useCallback } from "react";
import { useScan } from "./hooks/useScan";
import { useClean } from "./hooks/useClean";
import { useVersion } from "./hooks/useVersion";
import { ProgressBar } from "./components/ProgressBar";
import { ScanView } from "./components/ScanView";
import { Settings } from "./components/Settings";
import { MapView } from "./components/MapView";
import { TrashView } from "./components/TrashView";
import { ScheduleView } from "./components/ScheduleView";
import { useI18n } from "./i18n";
import type { ScanConfig } from "./types";
import { formatSize } from "./utils";

type ScanSpeed = "quick" | "normal" | "thorough";
type Tab = "scan" | "map" | "trash" | "schedule";

// Default scan settings
const DEFAULT_MIN_FILE_SIZE_MB = 100;

function App() {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<Tab>("scan");
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [speed, setSpeed] = useState<ScanSpeed>("normal");
  const [minFileSizeMb, setMinFileSizeMb] = useState(DEFAULT_MIN_FILE_SIZE_MB);
  const [findDuplicates, setFindDuplicates] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const { results, isScanning, progress: scanProgress, error: scanError, scan, clearResults } = useScan();
  const { isCleaning, progress: cleanProgress, result: cleanResult, error: cleanError, cleanItems, clearResult } = useClean();
  const { versionInfo } = useVersion();

  const handleScan = async () => {
    setSelectedPaths(new Set());
    clearResult();
    const config: ScanConfig = {
      speed,
      min_file_size_mb: minFileSizeMb,
      find_duplicates: findDuplicates,
    };
    try {
      await scan(config);
    } catch (e) {
      console.error("Scan failed:", e);
    }
  };

  const handleToggleItem = useCallback((path: string) => {
    setSelectedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const handleToggleCategory = useCallback(
    (category: string, selected: boolean) => {
      if (!results) return;

      setSelectedPaths((prev) => {
        const next = new Set(prev);
        const categoryItems = results.items.filter(
          (item) => item.category === category
        );

        for (const item of categoryItems) {
          if (selected) {
            next.add(item.path);
          } else {
            next.delete(item.path);
          }
        }

        return next;
      });
    },
    [results]
  );

  const handleClean = async () => {
    if (selectedPaths.size === 0) return;

    const paths = Array.from(selectedPaths);
    try {
      await cleanItems(paths, false);
      setSelectedPaths(new Set());
      await scan({ speed, min_file_size_mb: minFileSizeMb, find_duplicates: findDuplicates });
    } catch (e) {
      console.error("Clean failed:", e);
    }
  };

  const isWorking = isScanning || isCleaning;
  const error = scanError || cleanError;

  const speedLabels: Record<ScanSpeed, string> = {
    quick: t("quick"),
    normal: t("normal"),
    thorough: t("thorough"),
  };

  return (
    <div className="h-screen flex flex-col p-5">
      {/* Header */}
      <header className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <img
            src="/mascot.png"
            alt="Cleanser mascot"
            className="w-12 h-12 animate-bounce-subtle drop-shadow-lg"
          />
          <div>
            <h1 className="text-2xl font-bold accent-text">Cleanser</h1>
            <span className="text-muted text-xs">{t("appTagline")}</span>
          </div>
        </div>

        <div className="flex items-center gap-3">
          {/* Version indicator */}
          {versionInfo && (
            <div className="flex items-center gap-2 text-xs">
              <span className="text-muted">v{versionInfo.current}</span>
              {versionInfo.update_available && versionInfo.release_url && (
                <a
                  href={versionInfo.release_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-1 px-2 py-1 rounded-full bg-[var(--orange-primary)]/10 text-[var(--orange-primary)] hover:bg-[var(--orange-primary)]/20 transition-colors"
                >
                  <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                  </svg>
                  v{versionInfo.latest}
                </a>
              )}
            </div>
          )}

          <button
            onClick={() => setShowSettings(true)}
            className="p-2.5 hover:bg-[var(--bg-secondary)] rounded-xl transition-colors"
            title={t("settings")}
            aria-label={t("settings")}
          >
            <svg
              className="w-5 h-5 text-secondary"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </button>
        </div>
      </header>

      {/* Tab navigation */}
      <div className="flex border-b border-[var(--border-color)] mb-4" role="tablist">
        <button
          onClick={() => setActiveTab("scan")}
          className={`tab-button ${activeTab === "scan" ? "active" : ""}`}
          aria-selected={activeTab === "scan"}
          role="tab"
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
            </svg>
            {t("scanTab")}
          </span>
        </button>
        <button
          onClick={() => setActiveTab("map")}
          className={`tab-button ${activeTab === "map" ? "active" : ""}`}
          aria-selected={activeTab === "map"}
          role="tab"
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7"/>
            </svg>
            {t("mapTab")}
          </span>
        </button>
        <button
          onClick={() => setActiveTab("trash")}
          className={`tab-button ${activeTab === "trash" ? "active" : ""}`}
          aria-selected={activeTab === "trash"}
          role="tab"
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
            </svg>
            {t("trashTab")}
          </span>
        </button>
        <button
          onClick={() => setActiveTab("schedule")}
          className={`tab-button ${activeTab === "schedule" ? "active" : ""}`}
          aria-selected={activeTab === "schedule"}
          role="tab"
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>
            {t("scheduleTab")}
          </span>
        </button>
      </div>

      {/* Main content */}
      {activeTab === "scan" ? (
        <>
          {/* Scan controls */}
          <div className="card p-5 mb-4">
            <div className="flex items-center gap-4">
              <button
                onClick={handleScan}
                disabled={isWorking}
                className="btn btn-primary flex items-center gap-2 px-6"
              >
                {isScanning ? (
                  <>
                    <svg className="w-5 h-5 animate-spin" viewBox="0 0 24 24">
                      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none"/>
                      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
                    </svg>
                    {t("scanning")}
                  </>
                ) : (
                  <>
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
                    </svg>
                    {t("scan")}
                  </>
                )}
              </button>

              <div className="flex items-center gap-2">
                <span className="text-sm text-muted">{t("speed")}</span>
                <div className="flex rounded-xl overflow-hidden border border-[var(--border-color)]">
                  {(["quick", "normal", "thorough"] as ScanSpeed[]).map((s) => (
                    <button
                      key={s}
                      onClick={() => setSpeed(s)}
                      disabled={isWorking}
                      className={`px-4 py-1.5 text-sm font-medium transition-all ${
                        speed === s
                          ? "bg-[var(--orange-primary)] text-white"
                          : "bg-[var(--bg-tertiary)] text-secondary hover:bg-[var(--bg-secondary)]"
                      }`}
                    >
                      {speedLabels[s]}
                    </button>
                  ))}
                </div>
              </div>

              {results && !isWorking && (
                <button
                  onClick={clearResults}
                  className="ml-auto text-sm text-muted hover:text-secondary transition-colors"
                >
                  {t("clearResults")}
                </button>
              )}
            </div>
          </div>

          {/* Progress bar */}
          {(isScanning || isCleaning) && (
            <ProgressBar
              scanProgress={scanProgress}
              cleanProgress={cleanProgress}
            />
          )}

          {/* Clean result notification */}
          {cleanResult && !isCleaning && (
            <div className="card p-4 mb-4 category-card-safe bg-safe/5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-full bg-safe/20 flex items-center justify-center">
                    <svg className="w-5 h-5 risk-safe" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7"/>
                    </svg>
                  </div>
                  <div>
                    <p className="font-semibold risk-safe">{t("cleanComplete")}</p>
                    <p className="text-sm text-secondary">
                      {t("itemsCleaned", { count: cleanResult.cleaned_count, size: formatSize(cleanResult.cleaned_size) })}
                    </p>
                    {cleanResult.failed_count > 0 && (
                      <p className="text-sm risk-risky">
                        {t("itemsFailed", { count: cleanResult.failed_count })}
                      </p>
                    )}
                  </div>
                </div>
                <button
                  onClick={clearResult}
                  className="text-muted hover:text-secondary p-1"
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12"/>
                  </svg>
                </button>
              </div>
            </div>
          )}

          {/* Error display */}
          {error && (
            <div className="card p-4 mb-4 category-card-risky bg-risky/5">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-risky/20 flex items-center justify-center">
                  <svg className="w-5 h-5 risk-risky" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                  </svg>
                </div>
                <p className="risk-risky flex-1">{error}</p>
              </div>
            </div>
          )}

          {/* Scan results or empty state */}
          <div className="flex-1 overflow-hidden">
            {results ? (
              <ScanView
                results={results}
                selectedPaths={selectedPaths}
                onToggleItem={handleToggleItem}
                onToggleCategory={handleToggleCategory}
                onClean={handleClean}
                isCleaning={isCleaning}
              />
            ) : (
              <div className="flex flex-col items-center justify-center h-full">
                <img
                  src="/mascot.png"
                  alt="Cleanser Mascot"
                  className="w-40 h-40 mb-6 animate-bounce-subtle drop-shadow-2xl"
                />
                <p className="text-xl font-semibold text-primary mb-2">
                  {t("readyToClean")}
                </p>
                <p className="text-muted">
                  {t("clickToScan")}
                </p>
              </div>
            )}
          </div>
        </>
      ) : activeTab === "map" ? (
        <div className="flex-1 overflow-hidden">
          <MapView />
        </div>
      ) : activeTab === "trash" ? (
        <div className="flex-1 overflow-hidden">
          <TrashView />
        </div>
      ) : (
        <div className="flex-1 overflow-hidden">
          <ScheduleView />
        </div>
      )}

      {/* Settings modal */}
      {showSettings && (
        <Settings
          onClose={() => setShowSettings(false)}
          minFileSizeMb={minFileSizeMb}
          onMinFileSizeMbChange={setMinFileSizeMb}
          findDuplicates={findDuplicates}
          onFindDuplicatesChange={setFindDuplicates}
        />
      )}
    </div>
  );
}

export default App;
