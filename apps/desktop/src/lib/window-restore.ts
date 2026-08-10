export function restoreWindowError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error ?? "");
  if (message.includes("No available grid position")) {
    return "没有可用位置，请先折叠一个窗口";
  }
  return "无法打开窗口";
}
