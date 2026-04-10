import {
  callVideoStart,
  callVideoPause,
  callVideoResume,
  callVideoStop,
  callCheckFfmpeg,
  callDownloadFfmpeg,
} from "../shared/invoke";

export async function startVideoRecording(
  x: number, y: number, width: number, height: number,
  quality?: { fps: number; crf: number; preset: string },
): Promise<void> {
  return callVideoStart({ x, y, width, height }, quality);
}

export {
  callVideoPause as pauseVideo,
  callVideoResume as resumeVideo,
  callVideoStop as stopVideo,
  callCheckFfmpeg as checkFfmpeg,
  callDownloadFfmpeg as downloadFfmpeg,
};
