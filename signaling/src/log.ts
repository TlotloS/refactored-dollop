export function log(level: "info" | "warn" | "error", msg: string, extra?: Record<string, unknown>): void {
  const line = {
    ts: new Date().toISOString(),
    level,
    msg,
    ...extra,
  };
  const out = level === "error" ? process.stderr : process.stdout;
  out.write(JSON.stringify(line) + "\n");
}
