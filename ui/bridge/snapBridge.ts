import { callSnap } from "../shared/invoke";

function monitorIndexFromUrl(): number | undefined {
  const m = new URLSearchParams(window.location.search).get("monitor");
  if (m === null) return undefined;
  const n = parseInt(m, 10);
  return Number.isNaN(n) ? undefined : n;
}

export async function takeSnap(
  x: number, y: number, width: number, height: number
): Promise<string> {
  const monitor_index = monitorIndexFromUrl();
  return callSnap({
    x, y, width, height,
    ...(monitor_index !== undefined ? { monitor_index } : {}),
  });
}

