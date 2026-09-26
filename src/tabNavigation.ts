export type TabNavigationKey = "ArrowLeft" | "ArrowRight" | "Home" | "End";

export function nextTabIndex(current: number, total: number, key: TabNavigationKey) {
  if (total <= 0) return 0;
  if (key === "Home") return 0;
  if (key === "End") return total - 1;
  if (key === "ArrowLeft") return (current - 1 + total) % total;
  return (current + 1) % total;
}
