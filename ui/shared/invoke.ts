import { invoke } from "@tauri-apps/api/core";

export interface MonitorInfo {
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
  is_primary: boolean;
}

export async function callGetScreenInfo(): Promise<MonitorInfo[]> {
  return invoke<MonitorInfo[]>("get_screen_info");
}

export async function callSnap(region: {
  x: number; y: number; width: number; height: number;
  monitor_index?: number;
}): Promise<string> {
  return invoke<string>("snap_region", { region });
}

export async function callGifStart(region: {
  x: number; y: number; width: number; height: number;
  monitor_index?: number;
}): Promise<void> {
  return invoke("gif_start", { region });
}

export async function callGifPause(): Promise<void> {
  return invoke("gif_pause");
}

export async function callGifResume(): Promise<void> {
  return invoke("gif_resume");
}

export async function callGifStop(): Promise<string> {
  return invoke<string>("gif_stop");
}

export async function callCheckFfmpeg(): Promise<boolean> {
  return invoke<boolean>("check_ffmpeg");
}

export async function callDownloadFfmpeg(): Promise<string> {
  return invoke<string>("download_ffmpeg");
}

export async function callVideoStart(
  region: {
    x: number; y: number; width: number; height: number;
    monitor_index?: number;
  },
  quality?: { fps: number; crf: number; preset: string; label?: string },
): Promise<void> {
  return invoke("video_start", { region, quality });
}

export async function callVideoPause(): Promise<void> {
  return invoke("video_pause");
}

export async function callVideoResume(): Promise<void> {
  return invoke("video_resume");
}

export async function callVideoStop(): Promise<void> {
  return invoke("video_stop");
}
