import { invoke } from "@tauri-apps/api/core";

export async function callSnap(region: {
  x: number; y: number; width: number; height: number;
}): Promise<string> {
  return invoke<string>("snap_region", { region });
}

export async function callGifStart(region: {
  x: number; y: number; width: number; height: number;
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
