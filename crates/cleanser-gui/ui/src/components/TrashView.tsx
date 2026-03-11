import { useEffect } from "react";
import { useAppStore } from "../store/appStore";
import { useI18n } from "../i18n";
import { formatSize } from "../utils";

export function TrashView() {
  const { t } = useI18n();
  const {
    trashItems,
    trashStats,
    isLoadingTrash,
    trashError,
    loadTrash,
    restoreTrashItem,
    deleteTrashItem,
    emptyTrash,
  } = useAppStore();

  useEffect(() => {
    loadTrash();
  }, [loadTrash]);

  const handleRestore = async (id: string) => {
    try {
      await restoreTrashItem(id);
    } catch (e) {
      console.error("Failed to restore item:", e);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteTrashItem(id);
    } catch (e) {
      console.error("Failed to delete item:", e);
    }
  };

  const handleEmptyTrash = async () => {
    if (!confirm(t("emptyTrashConfirm", { count: trashItems.length }))) return;
    try {
      await emptyTrash();
    } catch (e) {
      console.error("Failed to empty trash:", e);
    }
  };

  if (isLoadingTrash) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted">{t("loading")}</div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="card p-4 mb-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold flex items-center gap-2">
              <svg className="w-5 h-5 text-[var(--orange-primary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
              </svg>
              {t("trashTab")}
            </h2>
            {trashStats && (
              <p className="text-sm text-muted mt-1">
                {t("trashItems", { count: trashStats.item_count })} • {t("trashSize", { size: formatSize(trashStats.total_size) })}
              </p>
            )}
          </div>
          {trashItems.length > 0 && (
            <button
              onClick={handleEmptyTrash}
              className="btn bg-[var(--risk-risky)] text-white hover:opacity-90"
            >
              {t("emptyTrash")}
            </button>
          )}
        </div>
      </div>

      {/* Error */}
      {trashError && (
        <div className="card p-4 mb-4 category-card-risky bg-risky/5">
          <p className="risk-risky">{trashError}</p>
        </div>
      )}

      {/* Content */}
      {trashItems.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center">
          <svg className="w-16 h-16 text-muted mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
          </svg>
          <p className="text-lg font-medium text-primary mb-1">{t("trashEmpty")}</p>
          <p className="text-sm text-muted">{t("trashEmptyDescription")}</p>
        </div>
      ) : (
        <div className="flex-1 overflow-auto space-y-2">
          {trashItems.map((item) => (
            <div key={item.id} className="card p-4 flex items-center gap-4">
              <div className="w-10 h-10 rounded-lg bg-[var(--bg-secondary)] flex items-center justify-center">
                {item.is_directory ? (
                  <svg className="w-5 h-5 text-[var(--orange-primary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                  </svg>
                ) : (
                  <svg className="w-5 h-5 text-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                  </svg>
                )}
              </div>
              <div className="flex-1 min-w-0">
                <p className="font-medium truncate" title={item.original_path}>
                  {item.original_path.split("/").pop() || item.original_path}
                </p>
                <p className="text-sm text-muted truncate" title={item.original_path}>
                  {item.original_path}
                </p>
                <p className="text-xs text-muted">
                  {formatSize(item.size)} • {item.age} ago
                </p>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => handleRestore(item.id)}
                  className="btn btn-secondary text-sm px-3 py-1.5"
                  title={t("restore")}
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"/>
                  </svg>
                </button>
                <button
                  onClick={() => handleDelete(item.id)}
                  className="btn text-sm px-3 py-1.5 bg-[var(--risk-risky)]/10 text-[var(--risk-risky)] hover:bg-[var(--risk-risky)]/20"
                  title={t("deleteForever")}
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12"/>
                  </svg>
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
