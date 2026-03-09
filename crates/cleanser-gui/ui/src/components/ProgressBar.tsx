import type { ScanProgress, CleanProgress } from "../types";
import { formatSize } from "../utils";

interface Props {
  scanProgress?: ScanProgress | null;
  cleanProgress?: CleanProgress | null;
}

export function ProgressBar({ scanProgress, cleanProgress }: Props) {
  const progress = scanProgress || cleanProgress;
  if (!progress) return null;

  const hasPercentage =
    progress.current !== null &&
    progress.total !== null &&
    progress.total > 0;
  const percentage = hasPercentage
    ? Math.round(((progress.current ?? 0) / (progress.total ?? 1)) * 100)
    : null;

  return (
    <div className="card p-4 mb-4">
      <div className="flex items-center justify-between mb-3">
        <span className="text-sm font-semibold accent-text">
          {progress.phase.replace(/([A-Z])/g, " $1").trim()}
        </span>
        {percentage !== null && (
          <span className="text-sm text-[var(--text-secondary)] font-mono">
            {percentage}%
          </span>
        )}
      </div>

      <div className="progress-bar h-2 mb-3">
        <div
          className="progress-bar-fill h-full"
          style={{
            width: percentage !== null ? `${percentage}%` : "100%",
            animation:
              percentage === null ? "pulse-orange 1.5s ease-in-out infinite" : "none",
          }}
        />
      </div>

      <p className="text-sm text-[var(--text-secondary)] truncate font-mono">
        {progress.message}
      </p>

      {cleanProgress && cleanProgress.cleaned_size > 0 && (
        <p className="text-sm risk-safe mt-2 font-semibold">
          Liberado: {formatSize(cleanProgress.cleaned_size)}
        </p>
      )}
    </div>
  );
}
