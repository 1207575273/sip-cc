import { startVideoRecording } from "../bridge/videoBridge";
import { mountSelectionOverlay } from "./SelectionOverlay";

interface VideoPreset {
  label: string;
  fps: number;
  crf: number;
  preset: string;
}

const PRESETS: VideoPreset[] = [
  { label: "流畅", fps: 10, crf: 28, preset: "veryfast" },
  { label: "标准", fps: 15, crf: 23, preset: "medium" },
  { label: "高清", fps: 30, crf: 18, preset: "medium" },
];

export function mountVideoOverlay(container: HTMLElement): () => void {
  let selected = 1;

  return mountSelectionOverlay(container, {
    color: "#4a90d9",
    buttonText: "开始录制",
    toolbarWidth: 340,
    onConfirm: (x, y, w, h) => {
      const p = PRESETS[selected];
      return startVideoRecording(x, y, w, h, {
        fps: p.fps,
        crf: p.crf,
        preset: p.preset,
        label: p.label,
      });
    },
    renderExtra: (slot) => {
      const group = document.createElement("div");
      group.className = "preset-group";
      PRESETS.forEach((p, i) => {
        const btn = document.createElement("button");
        btn.className = `preset-btn${i === selected ? " preset-active" : ""}`;
        btn.textContent = p.label;
        btn.addEventListener("click", () => {
          selected = i;
          group.querySelectorAll(".preset-btn").forEach((b, j) => {
            b.classList.toggle("preset-active", j === i);
          });
        });
        group.appendChild(btn);
      });
      slot.appendChild(group);
    },
  });
}
