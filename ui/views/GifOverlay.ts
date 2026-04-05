import { startGifRecording } from "../bridge/gifBridge";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface Rect {
  startX: number; startY: number; endX: number; endY: number;
}

export function mountGifOverlay(container: HTMLElement): void {
  const canvas = document.createElement("canvas");
  canvas.id = "gif-canvas";
  container.appendChild(canvas);

  const ctx = canvas.getContext("2d")!;
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;

  let isDragging = false;
  let rect: Rect = { startX: 0, startY: 0, endX: 0, endY: 0 };

  function drawOverlay(): void {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "rgba(0, 0, 0, 0.4)";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    if (isDragging) {
      const x = Math.min(rect.startX, rect.endX);
      const y = Math.min(rect.startY, rect.endY);
      const w = Math.abs(rect.endX - rect.startX);
      const h = Math.abs(rect.endY - rect.startY);

      ctx.clearRect(x, y, w, h);
      ctx.strokeStyle = "#ff4444";
      ctx.lineWidth = 2;
      ctx.strokeRect(x, y, w, h);

      ctx.fillStyle = "#ff4444";
      ctx.font = "14px monospace";
      ctx.fillText(`${w} × ${h}`, x, y > 20 ? y - 6 : y + h + 16);
    }
  }

  canvas.addEventListener("mousedown", (e: MouseEvent) => {
    isDragging = true;
    rect.startX = e.clientX; rect.startY = e.clientY;
    rect.endX = e.clientX; rect.endY = e.clientY;
  });

  canvas.addEventListener("mousemove", (e: MouseEvent) => {
    if (!isDragging) return;
    rect.endX = e.clientX; rect.endY = e.clientY;
    drawOverlay();
  });

  canvas.addEventListener("mouseup", async () => {
    isDragging = false;
    const x = Math.min(rect.startX, rect.endX);
    const y = Math.min(rect.startY, rect.endY);
    const w = Math.abs(rect.endX - rect.startX);
    const h = Math.abs(rect.endY - rect.startY);

    if (w > 5 && h > 5) {
      await startGifRecording(x, y, w, h);
    }
    const win = getCurrentWindow();
    await win.close();
  });

  canvas.addEventListener("keydown", async (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      const win = getCurrentWindow();
      await win.close();
    }
  });

  canvas.setAttribute("tabindex", "0");
  canvas.focus();
  drawOverlay();
}
