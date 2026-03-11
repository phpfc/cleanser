import { create } from "zustand";
import { persist, devtools } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  ScanConfig,
  ScanResults,
  ScanProgress,
  CleanResult,
  CleanProgress,
  TrashEntry,
  TrashStats,
  ScheduledJob,
  CreateJobInput,
} from "../types";

interface AppState {
  // Scan state
  scanResults: ScanResults | null;
  isScanning: boolean;
  scanProgress: ScanProgress | null;
  scanError: string | null;

  // Clean state
  isCleaning: boolean;
  cleanProgress: CleanProgress | null;
  cleanResult: CleanResult | null;
  cleanError: string | null;

  // Selection state
  selectedPaths: Set<string>;

  // Settings state
  minFileSizeMb: number;
  findDuplicates: boolean;

  // Trash state
  trashItems: TrashEntry[];
  trashStats: TrashStats | null;
  isLoadingTrash: boolean;
  trashError: string | null;

  // Schedule state
  scheduledJobs: ScheduledJob[];
  isLoadingJobs: boolean;
  jobsError: string | null;

  // Scan actions
  scan: (config: ScanConfig) => Promise<void>;
  clearScanResults: () => void;
  removeItems: (paths: string[]) => void;

  // Clean actions
  cleanItems: (paths: string[], dryRun?: boolean) => Promise<void>;
  clearCleanResult: () => void;

  // Selection actions
  toggleItem: (path: string) => void;
  toggleCategory: (category: string, selected: boolean) => void;
  selectAll: () => void;
  deselectAll: () => void;

  // Settings actions
  setMinFileSizeMb: (value: number) => void;
  setFindDuplicates: (value: boolean) => void;
  addToWhitelist: (path: string) => Promise<void>;

  // Trash actions
  loadTrash: () => Promise<void>;
  restoreTrashItem: (id: string) => Promise<void>;
  deleteTrashItem: (id: string) => Promise<void>;
  emptyTrash: () => Promise<void>;

  // Schedule actions
  loadJobs: () => Promise<void>;
  createJob: (job: CreateJobInput) => Promise<void>;
  toggleJob: (job: ScheduledJob) => Promise<void>;
  removeJob: (jobName: string) => Promise<void>;

  // Initialization
  initialize: () => Promise<void>;
}

export const useAppStore = create<AppState>()(
  devtools(
    persist(
      (set, get) => ({
  // Initial state
  scanResults: null,
  isScanning: false,
  scanProgress: null,
  scanError: null,

  isCleaning: false,
  cleanProgress: null,
  cleanResult: null,
  cleanError: null,

  selectedPaths: new Set(),

  minFileSizeMb: 100,
  findDuplicates: false,

  trashItems: [],
  trashStats: null,
  isLoadingTrash: false,
  trashError: null,

  scheduledJobs: [],
  isLoadingJobs: false,
  jobsError: null,

  // Scan actions
  scan: async (config: ScanConfig) => {
    set({
      isScanning: true,
      scanError: null,
      scanProgress: null,
      selectedPaths: new Set(),
      cleanResult: null,
    });

    try {
      const scanResults = await invoke<ScanResults>("scan", { config });
      set({ scanResults, isScanning: false, scanProgress: null });
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ scanError: errorMsg, isScanning: false, scanProgress: null });
      throw e;
    }
  },

  clearScanResults: () => {
    set({ scanResults: null, scanError: null, selectedPaths: new Set() });
  },

  removeItems: (paths: string[]) => {
    const { scanResults } = get();
    if (!scanResults) return;

    const pathSet = new Set(paths);
    const filteredItems = scanResults.items.filter(
      (item) => !pathSet.has(item.path)
    );

    // If all items were removed, clear results
    if (filteredItems.length === 0) {
      set({ scanResults: null, selectedPaths: new Set() });
      return;
    }

    // Recalculate totals
    const totalSize = filteredItems.reduce((sum, item) => sum + item.size, 0);

    set({
      scanResults: {
        ...scanResults,
        items: filteredItems,
        items_count: filteredItems.length,
        total_size: totalSize,
      },
    });
  },

  // Clean actions
  cleanItems: async (paths: string[], dryRun: boolean = false) => {
    set({
      isCleaning: true,
      cleanError: null,
      cleanProgress: null,
      cleanResult: null,
    });

    try {
      const cleanResult = await invoke<CleanResult>("clean_items", {
        paths,
        dryRun,
      });
      set({ cleanResult, isCleaning: false, cleanProgress: null });

      // Remove cleaned items from scan results
      get().removeItems(paths);
      // Clear selection
      set({ selectedPaths: new Set() });
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ cleanError: errorMsg, isCleaning: false, cleanProgress: null });
      throw e;
    }
  },

  clearCleanResult: () => {
    set({ cleanResult: null, cleanError: null });
  },

  // Selection actions
  toggleItem: (path: string) => {
    const { selectedPaths } = get();
    const newSelection = new Set(selectedPaths);

    if (newSelection.has(path)) {
      newSelection.delete(path);
    } else {
      newSelection.add(path);
    }

    set({ selectedPaths: newSelection });
  },

  toggleCategory: (category: string, selected: boolean) => {
    const { scanResults, selectedPaths } = get();
    if (!scanResults) return;

    const newSelection = new Set(selectedPaths);
    const categoryItems = scanResults.items.filter(
      (item) => item.category === category
    );

    for (const item of categoryItems) {
      if (selected) {
        newSelection.add(item.path);
      } else {
        newSelection.delete(item.path);
      }
    }

    set({ selectedPaths: newSelection });
  },

  selectAll: () => {
    const { scanResults } = get();
    if (!scanResults) return;

    const allPaths = new Set(scanResults.items.map((item) => item.path));
    set({ selectedPaths: allPaths });
  },

  deselectAll: () => {
    set({ selectedPaths: new Set() });
  },

  // Settings actions
  setMinFileSizeMb: (value: number) => {
    set({ minFileSizeMb: value });
  },

  setFindDuplicates: (value: boolean) => {
    set({ findDuplicates: value });
  },

  addToWhitelist: async (path: string) => {
    try {
      await invoke("add_to_whitelist", { path });

      // Remove from scan results
      get().removeItems([path]);

      // Remove from selection if selected
      const { selectedPaths } = get();
      if (selectedPaths.has(path)) {
        const newSelection = new Set(selectedPaths);
        newSelection.delete(path);
        set({ selectedPaths: newSelection });
      }
    } catch (e) {
      console.error("Failed to add to whitelist:", e);
      throw e;
    }
  },

  // Trash actions
  loadTrash: async () => {
    set({ isLoadingTrash: true, trashError: null });
    try {
      const [items, stats] = await Promise.all([
        invoke<TrashEntry[]>("get_trash_items"),
        invoke<TrashStats>("get_trash_stats"),
      ]);
      set({ trashItems: items, trashStats: stats, isLoadingTrash: false });
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ trashError: errorMsg, isLoadingTrash: false });
      throw e;
    }
  },

  restoreTrashItem: async (id: string) => {
    try {
      await invoke("restore_trash_item", { entryId: id, toPath: null });
      // Optimistically remove from list
      set((state) => ({
        trashItems: state.trashItems.filter((item) => item.id !== id),
      }));
      // Reload to get updated stats
      await get().loadTrash();
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ trashError: errorMsg });
      throw e;
    }
  },

  deleteTrashItem: async (id: string) => {
    try {
      await invoke("delete_trash_item", { entryId: id });
      // Optimistically remove from list
      set((state) => ({
        trashItems: state.trashItems.filter((item) => item.id !== id),
      }));
      // Reload to get updated stats
      await get().loadTrash();
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ trashError: errorMsg });
      throw e;
    }
  },

  emptyTrash: async () => {
    try {
      await invoke("empty_trash");
      set({ trashItems: [], trashStats: null });
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ trashError: errorMsg });
      throw e;
    }
  },

  // Schedule actions
  loadJobs: async () => {
    set({ isLoadingJobs: true, jobsError: null });
    try {
      const jobs = await invoke<ScheduledJob[]>("get_scheduled_jobs");
      set({ scheduledJobs: jobs, isLoadingJobs: false });
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ jobsError: errorMsg, isLoadingJobs: false });
      throw e;
    }
  },

  createJob: async (job: CreateJobInput) => {
    try {
      await invoke("create_scheduled_job", { job });
      await get().loadJobs();
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ jobsError: errorMsg });
      throw e;
    }
  },

  toggleJob: async (job: ScheduledJob) => {
    try {
      if (job.enabled) {
        await invoke("disable_scheduled_job", { jobName: job.name });
      } else {
        await invoke("enable_scheduled_job", { jobName: job.name });
      }
      // Optimistically update
      set((state) => ({
        scheduledJobs: state.scheduledJobs.map((j) =>
          j.id === job.id ? { ...j, enabled: !j.enabled } : j
        ),
      }));
      await get().loadJobs();
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ jobsError: errorMsg });
      throw e;
    }
  },

  removeJob: async (jobName: string) => {
    try {
      await invoke("remove_scheduled_job", { jobName });
      // Optimistically remove
      set((state) => ({
        scheduledJobs: state.scheduledJobs.filter((j) => j.name !== jobName),
      }));
      await get().loadJobs();
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e);
      set({ jobsError: errorMsg });
      throw e;
    }
  },

  // Initialization
  initialize: async () => {
    // Load cached scan results on startup
    try {
      const cached = await invoke<ScanResults | null>("get_cached_scan");
      if (cached) {
        set({ scanResults: cached });
      }
    } catch (e) {
      console.error("Failed to load cached scan:", e);
    }

    // Listen to scan progress events
    listen<ScanProgress>("scan:progress", (event) => {
      set({ scanProgress: event.payload });
    });

    // Listen to clean progress events
    listen<CleanProgress>("clean:progress", (event) => {
      set({ cleanProgress: event.payload });
    });
  },
}),
      {
        name: "cleanser-settings",
        // Persist user settings and last scan results for faster startup
        partialize: (state) => ({
          minFileSizeMb: state.minFileSizeMb,
          findDuplicates: state.findDuplicates,
          scanResults: state.scanResults,
        }),
      }
    ),
    {
      name: "CleanserStore",
      // DevTools are always available in Zustand - browser extension will detect them
    }
  )
);
