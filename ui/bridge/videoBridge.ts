import {
  callVideoStart,
  callVideoPause,
  callVideoResume,
  callVideoStop,
  callCheckFfmpeg,
  callDownloadFfmpeg,
} from "../shared/invoke";

function monitorIndexFromUrl(): number | undefined {
  const m = new URLSearchParams(window.location.search).get("monitor");
  if (m === null) return undefined;
  const n = parseInt(m, 10);
  return Number.isNaN(n) ? undefined : n;
}

export async function startVideoRecording(
  x: number, y: number, width: number, height: number,
  quality?: { fps: number; crf: number; preset: string; label?: string },
): Promise<void> {
  const monitor_index = monitorIndexFromUrl();
  return callVideoStart(
    { x, y, width, height, ...(monitor_index !== undefined ? { monitor_index } : {}) },
    quality,
  );
}

export {
  callVideoPause as pauseVideo,
  callVideoResume as resumeVideo,
  callVideoStop as stopVideo,
  callCheckFfmpeg as checkFfmpeg,
  callDownloadFfmpeg as downloadFfmpeg,
};
