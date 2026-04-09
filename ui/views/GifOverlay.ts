import { startGifRecording } from "../bridge/gifBridge";
import { mountSelectionOverlay } from "./SelectionOverlay";

export function mountGifOverlay(container: HTMLElement): () => void {
  return mountSelectionOverlay(container, {
    color: "#ff4444",
    buttonText: "开始录制",
    toolbarWidth: 160,
    onConfirm: (x, y, w, h) => startGifRecording(x, y, w, h),
  });
}
