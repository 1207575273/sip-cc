import { takeSnap } from "../bridge/snapBridge";
import { mountSelectionOverlay } from "./SelectionOverlay";

export function mountSnapOverlay(container: HTMLElement): () => void {
  return mountSelectionOverlay(container, {
    color: "#00aaff",
    buttonText: "保存",
    toolbarWidth: 130,
    onConfirm: (x, y, w, h) => takeSnap(x, y, w, h),
  });
}
