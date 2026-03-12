import { useRef } from "react";
import type { ScanProgress, CleanProgress } from "../types";
import { formatSize } from "../utils";
import { useI18n } from "../i18n";

interface Props {
  scanProgress?: ScanProgress | null;
  cleanProgress?: CleanProgress | null;
}

export function ProgressBar({ scanProgress, cleanProgress }: Props) {
  const { t } = useI18n();
  const progress = scanProgress || cleanProgress;
  const lastPercentageRef = useRef<number>(0);
  const lastPhaseRef = useRef<string>("");

  if (!progress) return null;

  const hasPercentage =
    progress.current !== null &&
    progress.total !== null &&
    progress.total > 0;

  let percentage: number | null = null;
  if (hasPercentage) {
    const calculated = Math.round(((progress.current ?? 0) / (progress.total ?? 1)) * 100);
    percentage = Math.min(100, Math.max(0, calculated));

    if (progress.phase !== lastPhaseRef.current) {
      lastPhaseRef.current = progress.phase;
      lastPercentageRef.current = percentage;
    } else if (percentage >= lastPercentageRef.current) {
      lastPercentageRef.current = percentage;
    } else {
      percentage = lastPercentageRef.current;
    }
  } else if (lastPhaseRef.current === progress.phase && lastPercentageRef.current > 0) {
    percentage = lastPercentageRef.current;
  }

  const getPhaseTranslation = (phase: string): string => {
    const phaseMap: Record<string, string> = {
      "LoadingMap": t("scanPhaseLoadingMap"),
      "UpdatingMap": t("scanPhaseUpdatingMap"),
      "Scanning": t("scanPhaseScanning"),
      "FindingDuplicates": t("scanPhaseFindingDuplicates"),
      "Complete": t("scanPhaseComplete"),
      "Loading": t("cleanPhaseLoading"),
      "Cleaning": t("cleanPhaseCleaning"),
    };
    return phaseMap[phase] || phase;
  };

  const getMessageTranslation = (message: string): string => {
    if (message === "Starting filesystem scan...") {
      return t("scanMsgStarting");
    }
    if (message === "No filesystem map found. Creating initial map...") {
      return t("scanMsgNoMapFound");
    }
    if (message === "Updating filesystem map...") {
      return t("scanMsgUpdatingMap");
    }
    if (message === "Finding duplicate files...") {
      return t("scanMsgFindingDuplicates");
    }
    if (message === "Scanning files by size...") {
      return t("scanMsgScanningBySize");
    }
    if (message === "Scan complete!") {
      return t("scanMsgComplete");
    }

    let match = message.match(/^Scanning (\d+) directories\.\.\.$/);
    if (match) {
      return t("scanMsgScanningDirs").replace("{count}", match[1]);
    }

    match = message.match(/^Scanned (\d+) directories$/);
    if (match) {
      return t("scanMsgScannedDirs").replace("{count}", match[1]);
    }

    match = message.match(/^Scanning mapped directories\.\.\. (\d+)\/(\d+)$/);
    if (match) {
      return t("scanMsgScanningMappedDirs")
        .replace("{current}", match[1])
        .replace("{total}", match[2]);
    }

    match = message.match(/^Computing partial hashes for (\d+) files\.\.\.$/);
    if (match) {
      return t("scanMsgComputingPartialHashes").replace("{count}", match[1]);
    }

    match = message.match(/^Computing partial hashes\.\.\. (\d+)\/(\d+)$/);
    if (match) {
      return t("scanMsgComputingPartialProgress")
        .replace("{current}", match[1])
        .replace("{total}", match[2]);
    }

    match = message.match(/^Computing full hashes for (\d+) candidates\.\.\.$/);
    if (match) {
      return t("scanMsgComputingFullHashes").replace("{count}", match[1]);
    }

    match = message.match(/^Computing full hashes\.\.\. (\d+)\/(\d+)$/);
    if (match) {
      return t("scanMsgComputingFullProgress")
        .replace("{current}", match[1])
        .replace("{total}", match[2]);
    }

    return message;
  };

  return (
    <div className="card p-4 mb-4">
      <div className="flex items-center justify-between mb-3">
        <span className="text-sm font-semibold accent-text">
          {getPhaseTranslation(progress.phase)}
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
            width: percentage !== null ? `${percentage}%` : "0%",
            animation:
              percentage === null ? "pulse-orange 1.5s ease-in-out infinite" : "none",
          }}
        />
      </div>

      <p className="text-sm text-[var(--text-secondary)] truncate font-mono">
        {getMessageTranslation(progress.message)}
      </p>

      {cleanProgress && cleanProgress.cleaned_size > 0 && (
        <p className="text-sm risk-safe mt-2 font-semibold">
          Liberado: {formatSize(cleanProgress.cleaned_size)}
        </p>
      )}
    </div>
  );
}
