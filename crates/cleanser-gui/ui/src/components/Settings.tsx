import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { SystemInfo } from "../types";
import { useI18n, Language, Theme } from "../i18n";

interface Props {
  onClose: () => void;
  minFileSizeMb: number;
  onMinFileSizeMbChange: (value: number) => void;
  findDuplicates: boolean;
  onFindDuplicatesChange: (value: boolean) => void;
}

export function Settings({
  onClose,
  minFileSizeMb,
  onMinFileSizeMbChange,
  findDuplicates,
  onFindDuplicatesChange
}: Props) {
  const { t, language, setLanguage, theme, setTheme } = useI18n();
  const [whitelist, setWhitelist] = useState<string[]>([]);
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      const [wl, info] = await Promise.all([
        invoke<string[]>("get_whitelist"),
        invoke<SystemInfo>("get_system_info"),
      ]);
      setWhitelist(wl);
      setSystemInfo(info);
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  };

  const handleAddPath = async () => {
    try {
      // Open native folder picker
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("protectedPaths"),
      });

      if (selected && typeof selected === "string") {
        await invoke("add_to_whitelist", { path: selected });
        setWhitelist([...whitelist, selected]);
        setError(null);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemovePath = async (path: string) => {
    try {
      await invoke("remove_from_whitelist", { path });
      setWhitelist(whitelist.filter((p) => p !== path));
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRevealInFileManager = async (path: string) => {
    try {
      await invoke("reveal_in_file_manager", { path });
    } catch (e) {
      console.error("Failed to reveal in file manager:", e);
    }
  };

  const languageOptions: { value: Language; label: string }[] = [
    { value: "en", label: "English" },
    { value: "pt-BR", label: "Português (BR)" },
  ];

  const themeOptions: { value: Theme; label: string }[] = [
    { value: "system", label: t("themeSystem") },
    { value: "light", label: t("themeLight") },
    { value: "dark", label: t("themeDark") },
  ];

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="card w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-[var(--border-color)]">
          <h2 className="text-lg font-bold text-primary">{t("settings")}</h2>
          <button
            onClick={onClose}
            className="p-1.5 hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors"
          >
            <svg
              className="w-5 h-5 text-secondary"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5">
          {/* Scan Settings */}
          <div className="mb-6">
            <h3 className="text-sm font-semibold text-secondary mb-3 uppercase tracking-wide">
              {t("scanSettings") || "Scan Settings"}
            </h3>
            <div className="space-y-4">
              <div>
                <label className="block text-xs text-muted mb-1">
                  {t("minFileSize") || "Minimum file size (MB)"}
                </label>
                <input
                  type="number"
                  min="1"
                  max="10000"
                  value={minFileSizeMb}
                  onChange={(e) => onMinFileSizeMbChange(Math.max(1, parseInt(e.target.value) || 100))}
                  className="w-full bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-xl px-3 py-2 text-sm focus:border-[var(--orange-primary)] focus:outline-none"
                />
                <p className="text-xs text-muted mt-1">
                  {t("minFileSizeDescription") || "Only detect files larger than this size"}
                </p>
              </div>
              <div className="flex items-center justify-between">
                <div>
                  <label className="text-sm text-primary">
                    {t("findDuplicates") || "Find duplicate files"}
                  </label>
                  <p className="text-xs text-muted">
                    {t("findDuplicatesDescription") || "Scan for duplicate files (slower)"}
                  </p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={findDuplicates}
                  onClick={() => onFindDuplicatesChange(!findDuplicates)}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    findDuplicates ? "bg-[var(--orange-primary)]" : "bg-[var(--bg-tertiary)]"
                  }`}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      findDuplicates ? "translate-x-6" : "translate-x-1"
                    }`}
                  />
                </button>
              </div>
            </div>
          </div>

          {/* Language & Theme */}
          <div className="mb-6">
            <h3 className="text-sm font-semibold text-secondary mb-3 uppercase tracking-wide">
              {t("language")} & {t("theme")}
            </h3>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-muted mb-1">
                  {t("language")}
                </label>
                <select
                  value={language}
                  onChange={(e) => setLanguage(e.target.value as Language)}
                  className="w-full bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-xl px-3 py-2 text-sm focus:border-[var(--orange-primary)] focus:outline-none"
                >
                  {languageOptions.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-xs text-muted mb-1">
                  {t("theme")}
                </label>
                <select
                  value={theme}
                  onChange={(e) => setTheme(e.target.value as Theme)}
                  className="w-full bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-xl px-3 py-2 text-sm focus:border-[var(--orange-primary)] focus:outline-none"
                >
                  {themeOptions.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </div>

          {/* System info */}
          {systemInfo && (
            <div className="mb-6">
              <h3 className="text-sm font-semibold text-secondary mb-3 uppercase tracking-wide">
                {t("systemInfo")}
              </h3>
              <div className="bg-[var(--bg-primary)] rounded-xl p-4 text-sm font-mono">
                <p className="mb-1">
                  <span className="text-muted">{t("platform")}</span>{" "}
                  <span className="accent-text">{systemInfo.platform}</span>
                </p>
                <p>
                  <span className="text-muted">{t("home")}</span>{" "}
                  <span className="text-primary">{systemInfo.home_dir}</span>
                </p>
              </div>
            </div>
          )}

          {/* Whitelist */}
          <div>
            <h3 className="text-sm font-semibold text-secondary mb-2 uppercase tracking-wide">
              {t("protectedPaths")}
            </h3>
            <p className="text-xs text-muted mb-4">
              {t("whitelistDescription")}
            </p>

            {/* Add new path button */}
            <button
              onClick={handleAddPath}
              className="w-full btn btn-secondary mb-4 flex items-center justify-center gap-2"
            >
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                />
              </svg>
              {t("add")}
            </button>

            {error && (
              <p className="text-sm risk-risky mb-3">{error}</p>
            )}

            {/* Whitelist items */}
            <div className="space-y-2">
              {whitelist.length === 0 ? (
                <p className="text-sm text-muted italic py-4 text-center">
                  {t("noWhitelistPaths")}
                </p>
              ) : (
                whitelist.map((path) => (
                  <div
                    key={path}
                    className="flex items-center justify-between bg-[var(--bg-primary)] rounded-xl px-4 py-3 group hover-card"
                  >
                    <span className="text-sm font-mono truncate flex-1 mr-2 text-primary" title={path}>
                      {path}
                    </span>
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => handleRevealInFileManager(path)}
                        className="p-1.5 hover:bg-[var(--bg-tertiary)] rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                        title={t("openInFileManager")}
                      >
                        <svg
                          className="w-4 h-4 text-muted"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                          />
                        </svg>
                      </button>
                      <button
                        onClick={() => handleRemovePath(path)}
                        className="p-1.5 hover:bg-[var(--bg-tertiary)] rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                        title={t("removeFromWhitelist")}
                      >
                        <svg
                          className="w-4 h-4 risk-risky"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                          />
                        </svg>
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
