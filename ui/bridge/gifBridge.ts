import { callGifStart, callGifPause, callGifResume, callGifStop } from "../shared/invoke";

export async function startGifRecording(
  x: number, y: number, width: number, height: number
): Promise<void> {
  return callGifStart({ x, y, width, height });
}

export { callGifPause as pauseGif, callGifResume as resumeGif, callGifStop as stopGif };
