import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CleanableItem } from "../types";
import { formatSize, truncatePath, getRiskTextClass, getRiskBadgeClass, getCategoryCardClass } from "../utils";
import { useI18n } from "../i18n";
import { WhitelistDirectoryModal } from "./WhitelistDirectoryModal";

interface Props {
  category: string;
  items: CleanableItem[];
  selectedPaths: Set<string>;
  onToggleItem: (path: string) => void;
  onToggleCategory: (category: string, selected: boolean) => void;
  onAddToWhitelist: (path: string) => void;
}

export function CategoryCard({
  category,
  items,
  selectedPaths,
  onToggleItem,
  onToggleCategory,
  onAddToWhitelist,
}: Props) {
  const { t } = useI18n();
  const [isExpanded, setIsExpanded] = useState(false);
  const [showWhitelistModal, setShowWhitelistModal] = useState(false);
  const [selectedPathForWhitelist, setSelectedPathForWhitelist] = useState<string | null>(null);

  const totalSize = items.reduce((sum, item) => sum + item.size, 0);
  const selectedCount = items.filter((item) =>
    selectedPaths.has(item.path)
  ).length;
  const allSelected = selectedCount === items.length;
  const someSelected = selectedCount > 0 && !allSelected;
  const riskLevel = items[0]?.risk_level ?? "safe";

  // Sort items by size descending
  const sortedItems = [...items].sort((a, b) => b.size - a.size);

  const handleRevealInFileManager = async (path: string) => {
    try {
      await invoke("reveal_in_file_manager", { path });
    } catch (e) {
      console.error("Failed to reveal in file manager:", e);
    }
  };

  const getRiskLabel = (risk: string) => {
    switch (risk) {
      case "safe": return t("safe");
      case "moderate": return t("moderate");
      case "risky": return t("risky");
      default: return risk;
    }
  };

  return (
    <div className={`card ${getCategoryCardClass(riskLevel)} mb-3 overflow-hidden`}>
      {/* Header */}
      <div
        className="flex items-center p-4 cursor-pointer hover-card"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <input
          type="checkbox"
          checked={allSelected}
          ref={(el) => {
            if (el) el.indeterminate = someSelected;
          }}
          onChange={(e) => {
            e.stopPropagation();
            onToggleCategory(category, !allSelected);
          }}
          onClick={(e) => e.stopPropagation()}
          className="mr-4"
        />

        <div className="flex-1">
          <div className="flex items-center gap-3 mb-1">
            <span className={`font-semibold text-lg ${getRiskTextClass(riskLevel)}`}>
              {t(category as any) || category}
            </span>
            <span className={`text-xs px-2 py-0.5 rounded-full ${getRiskBadgeClass(riskLevel)}`}>
              {getRiskLabel(riskLevel)}
            </span>
            <span className="text-muted text-sm">
              {items.length} {items.length === 1 ? t("item") : t("items")}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-primary font-mono font-semibold">
              {formatSize(totalSize)}
            </span>
            {selectedCount > 0 && selectedCount < items.length && (
              <span className="text-sm accent-text">
                ({selectedCount} {t("selected").toLowerCase()})
              </span>
            )}
          </div>
        </div>

        <svg
          className={`w-5 h-5 text-muted transition-transform duration-200 ${
            isExpanded ? "rotate-180" : ""
          }`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </div>

      {/* Expanded items list */}
      {isExpanded && (
        <div className="border-t border-[var(--border-color)] max-h-72 overflow-y-auto">
          {sortedItems.map((item) => (
            <div
              key={item.path}
              className="flex items-center p-3 px-4 hover-card group border-b border-[var(--border-color)] last:border-b-0"
            >
              <input
                type="checkbox"
                checked={selectedPaths.has(item.path)}
                onChange={() => onToggleItem(item.path)}
                className="mr-4"
              />

              <div className="flex-1 min-w-0">
                <p
                  className="text-sm truncate font-mono text-primary"
                  title={item.path}
                >
                  {truncatePath(item.path, 55)}
                </p>
                <p className="text-xs text-muted mt-0.5">
                  {item.description}
                </p>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-sm text-secondary font-mono whitespace-nowrap">
                  {formatSize(item.size)}
                </span>

                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleRevealInFileManager(item.path);
                  }}
                  className="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-[var(--bg-tertiary)] rounded-lg transition-all"
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
                  onClick={(e) => {
                    e.stopPropagation();
                    setSelectedPathForWhitelist(item.path);
                    setShowWhitelistModal(true);
                  }}
                  className="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-[var(--bg-tertiary)] rounded-lg transition-all"
                  title={t("addToWhitelist")}
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
                      d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
                    />
                  </svg>
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Whitelist Directory Modal */}
      {showWhitelistModal && selectedPathForWhitelist && (
        <WhitelistDirectoryModal
          path={selectedPathForWhitelist}
          onClose={() => {
            setShowWhitelistModal(false);
            setSelectedPathForWhitelist(null);
          }}
          onConfirm={(selectedPath) => {
            onAddToWhitelist(selectedPath);
            setShowWhitelistModal(false);
            setSelectedPathForWhitelist(null);
          }}
        />
      )}
    </div>
  );
}
