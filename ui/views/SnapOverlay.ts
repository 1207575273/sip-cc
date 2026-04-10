import { takeSnap } from "../bridge/snapBridge";
import { mountSelectionOverlay } from "./SelectionOverlay";

const IS_WIN =
  typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent);

export function mountSnapOverlay(container: HTMLElement): () => void {
  return mountSelectionOverlay(container, {
    color: "#00aaff",
    buttonText: "保存",
    toolbarWidth: IS_WIN ? 220 : 130,
    onConfirm: (x, y, w, h) => takeSnap(x, y, w, h),
    ...(IS_WIN
      ? {
          longSnapLabel: "长截图",
          longSnapSupported: true,
        }
      : {}),
  });
}
