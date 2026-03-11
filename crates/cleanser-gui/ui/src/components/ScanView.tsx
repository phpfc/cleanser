import { useState, useMemo } from "react";
import { useAppStore } from "../store/appStore";
import type { CleanableItem, CategoryGroup } from "../types";
import { CategoryCard } from "./CategoryCard";
import { formatSize } from "../utils";
import { useI18n } from "../i18n";

interface Props {
  onClean: () => void;
}

export function ScanView({ onClean }: Props) {
  const { t } = useI18n();
  const [sortBy, setSortBy] = useState<"size" | "count">("size");

  // Get state from Zustand store
  const {
    scanResults,
    selectedPaths,
    isCleaning,
    toggleItem,
    toggleCategory,
    selectAll,
    deselectAll,
    addToWhitelist,
  } = useAppStore();

  // Group items by category
  const categoryGroups = useMemo(() => {
    if (!scanResults) return [];

    const groups = new Map<string, CleanableItem[]>();
    for (const item of scanResults.items) {
      const existing = groups.get(item.category) || [];
      existing.push(item);
      groups.set(item.category, existing);
    }

    const categoryList: CategoryGroup[] = Array.from(groups.entries()).map(
      ([category, items]) => ({
        category,
        items,
        totalSize: items.reduce((sum, item) => sum + item.size, 0),
        riskLevel: items[0]?.risk_level ?? "safe",
      })
    );

    // Sort categories
    if (sortBy === "size") {
      categoryList.sort((a, b) => b.totalSize - a.totalSize);
    } else {
      categoryList.sort((a, b) => b.items.length - a.items.length);
    }

    return categoryList;
  }, [scanResults, sortBy]);

  // Calculate selected size
  const selectedSize = useMemo(() => {
    if (!scanResults) return 0;
    return scanResults.items
      .filter((item) => selectedPaths.has(item.path))
      .reduce((sum, item) => sum + item.size, 0);
  }, [scanResults, selectedPaths]);

  const handleAddToWhitelist = async (path: string) => {
    try {
      await addToWhitelist(path);
    } catch (e) {
      console.error("Failed to add to whitelist:", e);
    }
  };

  const handleSelectAll = () => {
    if (!scanResults) return;

    const allPaths = scanResults.items.map((item) => item.path);
    const allSelected = allPaths.every((path) => selectedPaths.has(path));

    if (allSelected) {
      deselectAll();
    } else {
      selectAll();
    }
  };

  if (!scanResults) return null;

  return (
    <div className="flex flex-col h-full">
      {/* Summary header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h2 className="text-xl font-bold text-primary">
            {t("found")} <span className="accent-text">{formatSize(scanResults.total_size)}</span>
          </h2>
          <p className="text-sm text-secondary">
            {scanResults.items_count} {scanResults.items_count === 1 ? t("item") : t("items")} {t("inCategories", { count: categoryGroups.length })}
          </p>
        </div>

        <div className="flex items-center gap-3">
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as "size" | "count")}
            className="bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-lg px-3 py-1.5 text-sm focus:border-[var(--orange-primary)] focus:outline-none"
          >
            <option value="size">{t("sortBySize")}</option>
            <option value="count">{t("sortByCount")}</option>
          </select>

          <button
            onClick={handleSelectAll}
            className="btn btn-secondary text-sm"
          >
            {selectedPaths.size === scanResults.items_count
              ? t("deselectAll")
              : t("selectAll")}
          </button>
        </div>
      </div>

      {/* Category list */}
      <div className="flex-1 overflow-y-auto pr-1">
        {categoryGroups.map((group) => (
          <CategoryCard
            key={group.category}
            category={group.category}
            items={group.items}
            selectedPaths={selectedPaths}
            onToggleItem={toggleItem}
            onToggleCategory={toggleCategory}
            onAddToWhitelist={handleAddToWhitelist}
          />
        ))}
      </div>

      {/* Action bar */}
      <div className="mt-4 pt-4 border-t border-[var(--border-color)] flex items-center justify-between">
        <div>
          <span className="text-secondary">{t("selected")} </span>
          <span className="font-bold accent-text">
            {selectedPaths.size} {selectedPaths.size === 1 ? t("item") : t("items")} ({formatSize(selectedSize)})
          </span>
        </div>

        <button
          onClick={onClean}
          disabled={selectedPaths.size === 0 || isCleaning}
          className="btn btn-danger px-6"
        >
          {isCleaning ? (
            <span className="flex items-center gap-2">
              <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none"/>
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
              </svg>
              {t("cleaning")}
            </span>
          ) : (
            t("cleanSelected")
          )}
        </button>
      </div>
    </div>
  );
}
