import { callGifStart, callGifPause, callGifResume, callGifStop } from "../shared/invoke";

function monitorIndexFromUrl(): number | undefined {
  const m = new URLSearchParams(window.location.search).get("monitor");
  if (m === null) return undefined;
  const n = parseInt(m, 10);
  return Number.isNaN(n) ? undefined : n;
}

export async function startGifRecording(
  x: number, y: number, width: number, height: number
): Promise<void> {
  const monitor_index = monitorIndexFromUrl();
  return callGifStart({
    x, y, width, height,
    ...(monitor_index !== undefined ? { monitor_index } : {}),
  });
}

export { callGifPause as pauseGif, callGifResume as resumeGif, callGifStop as stopGif };
