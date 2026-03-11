import { useState, useEffect } from "react";
import { useAppStore } from "../store/appStore";
import { useI18n } from "../i18n";
import type { CreateJobInput } from "../types";

export function ScheduleView() {
  const { t } = useI18n();
  const [showCreateModal, setShowCreateModal] = useState(false);

  const {
    scheduledJobs,
    isLoadingJobs,
    jobsError,
    loadJobs,
    toggleJob,
    removeJob,
  } = useAppStore();

  useEffect(() => {
    loadJobs();
  }, [loadJobs]);

  const handleToggle = async (job: typeof scheduledJobs[0]) => {
    try {
      await toggleJob(job);
    } catch (e) {
      console.error("Failed to toggle job:", e);
    }
  };

  const handleRemove = async (jobName: string) => {
    if (!confirm(`Remove scheduled job "${jobName}"?`)) return;
    try {
      await removeJob(jobName);
    } catch (e) {
      console.error("Failed to remove job:", e);
    }
  };

  if (isLoadingJobs) {
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
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
              </svg>
              {t("scheduleTab")}
            </h2>
            <p className="text-sm text-muted mt-1">
              {scheduledJobs.length} {scheduledJobs.length === 1 ? "job" : "jobs"}
            </p>
          </div>
          <button
            onClick={() => setShowCreateModal(true)}
            className="btn btn-primary"
          >
            {t("createJob")}
          </button>
        </div>
      </div>

      {/* Error */}
      {jobsError && (
        <div className="card p-4 mb-4 category-card-risky bg-risky/5">
          <p className="risk-risky">{jobsError}</p>
        </div>
      )}

      {/* Content */}
      {scheduledJobs.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center">
          <svg className="w-16 h-16 text-muted mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
          </svg>
          <p className="text-lg font-medium text-primary mb-1">{t("noScheduledJobs")}</p>
          <p className="text-sm text-muted">{t("noScheduledJobsDescription")}</p>
        </div>
      ) : (
        <div className="flex-1 overflow-auto space-y-2">
          {scheduledJobs.map((job) => (
            <div key={job.id} className="card p-4">
              <div className="flex items-center gap-4">
                <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${
                  job.enabled ? "bg-[var(--risk-safe)]/20" : "bg-[var(--bg-secondary)]"
                }`}>
                  <svg className={`w-5 h-5 ${job.enabled ? "text-[var(--risk-safe)]" : "text-muted"}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                  </svg>
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <p className="font-medium">{job.name}</p>
                    <span className={`text-xs px-2 py-0.5 rounded-full ${
                      job.enabled
                        ? "bg-[var(--risk-safe)]/20 text-[var(--risk-safe)]"
                        : "bg-[var(--bg-tertiary)] text-muted"
                    }`}>
                      {job.enabled ? t("enabled") : t("disabled")}
                    </span>
                  </div>
                  <p className="text-sm text-muted">{job.frequency}</p>
                  <div className="flex gap-3 mt-1 text-xs text-muted">
                    <span>Risk: {job.risk_level}</span>
                    {job.use_trash && <span>Trash</span>}
                    {job.secure_delete && <span>Secure</span>}
                    <span>{t("lastRun")}: {job.last_run || t("never")}</span>
                  </div>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={() => handleToggle(job)}
                    className={`btn text-sm px-3 py-1.5 ${
                      job.enabled
                        ? "bg-[var(--bg-secondary)] text-secondary"
                        : "bg-[var(--risk-safe)]/10 text-[var(--risk-safe)]"
                    }`}
                  >
                    {job.enabled ? t("disable") : t("enable")}
                  </button>
                  <button
                    onClick={() => handleRemove(job.name)}
                    className="btn text-sm px-3 py-1.5 bg-[var(--risk-risky)]/10 text-[var(--risk-risky)] hover:bg-[var(--risk-risky)]/20"
                  >
                    {t("remove")}
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Create Modal */}
      {showCreateModal && (
        <CreateJobModal
          onClose={() => setShowCreateModal(false)}
          onCreated={() => {
            setShowCreateModal(false);
            loadJobs();
          }}
        />
      )}
    </div>
  );
}

function CreateJobModal({ onClose, onCreated }: { onClose: () => void; onCreated: () => void }) {
  const { t } = useI18n();
  const { createJob } = useAppStore();
  const [name, setName] = useState("");
  const [frequency, setFrequency] = useState("daily");
  const [riskLevel, setRiskLevel] = useState("safe");
  const [useTrash, setUseTrash] = useState(false);
  const [secureDelete, setSecureDelete] = useState(false);
  const [notify, setNotify] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    if (!name.trim()) {
      setError("Name is required");
      return;
    }

    try {
      setCreating(true);
      const job: CreateJobInput = {
        name: name.trim(),
        frequency,
        risk_level: riskLevel,
        use_trash: useTrash,
        secure_delete: secureDelete,
        notify,
      };
      await createJob(job);
      onCreated();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div className="card p-6 w-full max-w-md" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-lg font-semibold mb-4">{t("createJob")}</h3>

        {error && (
          <div className="mb-4 p-3 rounded-lg bg-[var(--risk-risky)]/10 text-[var(--risk-risky)] text-sm">
            {error}
          </div>
        )}

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">{t("jobName")}</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border-color)] bg-[var(--bg-primary)]"
              placeholder="daily-cleanup"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1">{t("frequency")}</label>
            <select
              value={frequency}
              onChange={(e) => setFrequency(e.target.value)}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border-color)] bg-[var(--bg-primary)]"
            >
              <option value="daily">{t("frequencyDaily")}</option>
              <option value="weekly">{t("frequencyWeekly")}</option>
              <option value="monthly">{t("frequencyMonthly")}</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium mb-1">{t("riskLevel")}</label>
            <select
              value={riskLevel}
              onChange={(e) => setRiskLevel(e.target.value)}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border-color)] bg-[var(--bg-primary)]"
            >
              <option value="safe">{t("safe")}</option>
              <option value="moderate">{t("moderate")}</option>
              <option value="risky">{t("risky")}</option>
            </select>
          </div>

          <div className="space-y-2">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={useTrash}
                onChange={(e) => setUseTrash(e.target.checked)}
                className="rounded"
              />
              <span className="text-sm">{t("useTrash")}</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={secureDelete}
                onChange={(e) => setSecureDelete(e.target.checked)}
                className="rounded"
              />
              <span className="text-sm">{t("secureDelete")}</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={notify}
                onChange={(e) => setNotify(e.target.checked)}
                className="rounded"
              />
              <span className="text-sm">{t("notifyOnComplete")}</span>
            </label>
          </div>
        </div>

        <div className="flex justify-end gap-2 mt-6">
          <button onClick={onClose} className="btn btn-secondary">
            Cancel
          </button>
          <button
            onClick={handleCreate}
            disabled={creating}
            className="btn btn-primary"
          >
            {creating ? t("loading") : t("createJob")}
          </button>
        </div>
      </div>
    </div>
  );
}
