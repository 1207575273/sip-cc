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

/** 系统级鼠标穿透（长截图滑动下层内容时须为 true；关闭 overlay 时后端会复位） */
export async function callSetOverlayCursorPassthrough(passthrough: boolean): Promise<void> {
  return invoke("set_overlay_cursor_passthrough", { passthrough });
}

export async function callSnap(region: {
  x: number; y: number; width: number; height: number;
  monitor_index?: number;
}): Promise<string> {
  return invoke<string>("snap_region", { region });
}

/** 开始手动长截图会话：仅登记选区；首帧由前端取景框就绪后 `callLongSnapAppendFrame`（仅 Windows） */
export async function callLongSnapStart(region: {
  x: number; y: number; width: number; height: number;
  monitor_index?: number;
}): Promise<void> {
  return invoke("long_snap_start", { region });
}

export async function callLongSnapAppendFrame(): Promise<number> {
  return invoke<number>("long_snap_append_frame");
}

export async function callLongSnapFinish(): Promise<string> {
  return invoke<string>("long_snap_finish");
}

export async function callLongSnapCancel(): Promise<void> {
  return invoke("long_snap_cancel");
}

export async function callLongSnapSupported(): Promise<boolean> {
  return invoke<boolean>("long_snap_supported");
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
