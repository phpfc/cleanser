import { useState, useMemo } from "react";
import { useI18n } from "../i18n";

interface Props {
  path: string;
  onClose: () => void;
  onConfirm: (selectedPath: string) => void;
}

export function WhitelistDirectoryModal({ path, onClose, onConfirm }: Props) {
  const { t } = useI18n();

  // Parse path into directory levels
  const directoryLevels = useMemo(() => {
    const parts = path.split("/").filter(Boolean);
    const levels: { path: string; name: string; depth: number }[] = [];

    for (let i = 0; i < parts.length; i++) {
      const levelPath = "/" + parts.slice(0, i + 1).join("/");
      const name = parts[i];
      levels.push({
        path: levelPath,
        name,
        depth: i,
      });
    }

    return levels.reverse(); // Show most specific first
  }, [path]);

  const [selectedPath, setSelectedPath] = useState(path);

  const handleConfirm = () => {
    onConfirm(selectedPath);
    onClose();
  };

  const getWidth = (depth: number) => {
    const maxDepth = directoryLevels.length - 1;
    const reversedDepth = maxDepth - depth;
    // Most specific (depth 0 when reversed) = 50%, most broad = 100%
    const widthPercentage = 50 + (reversedDepth / maxDepth) * 50;
    return `${widthPercentage}%`;
  };

  return (
    <div
      className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="card w-full max-w-2xl mx-4 max-h-[80vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-[var(--border-color)]">
          <div className="flex-1">
            <h2 className="text-lg font-bold text-primary flex items-center gap-2">
              <svg className="w-5 h-5 text-[var(--orange-primary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
              </svg>
              {t("selectWhitelistLevel") || "Select Directory Level"}
            </h2>
            <p className="text-sm text-muted mt-1">
              {t("whitelistLevelDescription") || "Choose which directory level to protect from cleaning"}
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 hover:bg-[var(--bg-tertiary)] rounded-lg transition-colors ml-4"
            aria-label="Close"
          >
            <svg className="w-5 h-5 text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12"/>
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5">
          <div className="space-y-2 flex flex-col items-end">
            {directoryLevels.map((level, index) => {
              const isSelected = selectedPath === level.path;
              const isFile = index === 0; // First item (most specific) is the file/folder itself

              return (
                <div
                  key={level.path}
                  className={`
                    flex items-start p-3 rounded-xl cursor-pointer transition-all
                    ${isSelected
                      ? "bg-[var(--orange-primary)]/10 border-2 border-[var(--orange-primary)]"
                      : "bg-[var(--bg-secondary)] border-2 border-transparent hover:border-[var(--border-color)]"
                    }
                  `}
                  onClick={() => setSelectedPath(level.path)}
                  style={{ width: getWidth(level.depth) }}
                >
                  <input
                    type="radio"
                    checked={isSelected}
                    onChange={() => setSelectedPath(level.path)}
                    className="mt-0.5 mr-3"
                    onClick={(e) => e.stopPropagation()}
                  />

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1 flex-wrap">
                      {isFile ? (
                        <svg className="w-4 h-4 text-muted flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                        </svg>
                      ) : (
                        <svg className="w-4 h-4 text-[var(--orange-primary)] flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                        </svg>
                      )}
                      <span className={`font-medium ${isSelected ? "text-[var(--orange-primary)]" : "text-primary"}`}>
                        {level.name}
                      </span>
                      {index === 0 && (
                        <span className="text-xs px-2 py-0.5 rounded-full bg-[var(--orange-primary)]/20 text-[var(--orange-primary)] flex-shrink-0 whitespace-nowrap">
                          {t("specific") || "Specific"}
                        </span>
                      )}
                      {index === directoryLevels.length - 1 && (
                        <span className="text-xs px-2 py-0.5 rounded-full bg-blue-500/20 text-blue-500 flex-shrink-0 whitespace-nowrap">
                          {t("broad") || "Broad"}
                        </span>
                      )}
                    </div>
                    <p className="text-xs font-mono text-muted truncate" title={level.path}>
                      {level.path}
                    </p>
                    {index === 0 && (
                      <p className="text-xs text-muted mt-1">
                        {t("protectThisItem") || "Protect only this specific item"}
                      </p>
                    )}
                    {index > 0 && index < directoryLevels.length - 1 && (
                      <p className="text-xs text-muted mt-1">
                        {t("protectDirectory") || "Protect this directory and all its contents"}
                      </p>
                    )}
                    {index === directoryLevels.length - 1 && (
                      <p className="text-xs text-muted mt-1">
                        {t("protectTopLevel") || "Protect entire top-level directory (broad protection)"}
                      </p>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          {/* Helper info */}
          <div className="mt-6 p-4 bg-[var(--bg-secondary)] rounded-xl border border-[var(--border-color)]">
            <div className="flex gap-3">
              <svg className="w-5 h-5 text-[var(--orange-primary)] flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
              </svg>
              <div>
                <p className="text-sm font-medium text-primary mb-1">
                  {t("whitelistTip") || "💡 Tip"}
                </p>
                <p className="text-xs text-muted">
                  {t("whitelistTipDescription") || "Selecting a parent directory will protect all files and folders within it. More specific selections give you finer control."}
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex flex-col gap-3 p-5 border-t border-[var(--border-color)]">
          <div className="text-sm text-muted min-w-0">
            <span className="font-medium text-primary">Selected:</span>
            <span className="ml-2 font-mono truncate block">{selectedPath}</span>
          </div>
          <div className="flex gap-3 flex-shrink-0">
            <button
              onClick={onClose}
              className="btn btn-secondary px-6 flex-1 sm:flex-initial"
            >
              {t("cancel") || "Cancel"}
            </button>
            <button
              onClick={handleConfirm}
              className="btn btn-primary px-6 flex-1 sm:flex-initial flex items-center justify-center gap-2 whitespace-nowrap"
            >
              <svg className="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
              </svg>
              <span className="truncate">{t("addToWhitelist") || "Add to Whitelist"}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
