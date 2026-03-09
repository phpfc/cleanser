// Utility functions

export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KiB", "MiB", "GiB", "TiB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

export function groupByCategory<T extends { category: string }>(
  items: T[]
): Map<string, T[]> {
  const groups = new Map<string, T[]>();
  for (const item of items) {
    const existing = groups.get(item.category) || [];
    existing.push(item);
    groups.set(item.category, existing);
  }
  return groups;
}

export function truncatePath(path: string, maxLength: number = 50): string {
  if (path.length <= maxLength) return path;
  return "..." + path.slice(path.length - maxLength + 3);
}

export function getRiskTextClass(risk: "safe" | "moderate" | "risky"): string {
  switch (risk) {
    case "safe":
      return "risk-safe";
    case "moderate":
      return "risk-moderate";
    case "risky":
      return "risk-risky";
  }
}

export function getRiskBadgeClass(risk: "safe" | "moderate" | "risky"): string {
  switch (risk) {
    case "safe":
      return "risk-badge-safe";
    case "moderate":
      return "risk-badge-moderate";
    case "risky":
      return "risk-badge-risky";
  }
}

export function getCategoryCardClass(risk: "safe" | "moderate" | "risky"): string {
  switch (risk) {
    case "safe":
      return "category-card-safe";
    case "moderate":
      return "category-card-moderate";
    case "risky":
      return "category-card-risky";
  }
}

