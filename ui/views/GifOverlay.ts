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
  let hasSelection = false;
  let selX = 0, selY = 0, selW = 0, selH = 0;
  let rect: Rect = { startX: 0, startY: 0, endX: 0, endY: 0 };

  const toolbar = document.createElement("div");
  toolbar.className = "selection-toolbar";
  toolbar.style.display = "none";
  toolbar.innerHTML = `
    <button class="toolbar-btn toolbar-btn-save" id="btn-record">开始录制</button>
    <button class="toolbar-btn toolbar-btn-exit" id="btn-exit">退出</button>
  `;
  container.appendChild(toolbar);

  function drawOverlay(): void {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "rgba(0, 0, 0, 0.4)";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    const x = hasSelection ? selX : Math.min(rect.startX, rect.endX);
    const y = hasSelection ? selY : Math.min(rect.startY, rect.endY);
    const w = hasSelection ? selW : Math.abs(rect.endX - rect.startX);
    const h = hasSelection ? selH : Math.abs(rect.endY - rect.startY);

    if ((isDragging || hasSelection) && w > 0 && h > 0) {
      ctx.clearRect(x, y, w, h);
      ctx.strokeStyle = "#ff4444";
      ctx.lineWidth = 2;
      ctx.strokeRect(x, y, w, h);

      ctx.fillStyle = "#ff4444";
      ctx.font = "14px monospace";
      ctx.fillText(`${w} × ${h}`, x, y > 20 ? y - 6 : y + h + 16);
    }
  }

  function showToolbar(): void {
    const tbX = selX + selW - 160;
    const tbY = selY + selH + 8;
    toolbar.style.left = `${Math.max(0, tbX)}px`;
    toolbar.style.top = `${Math.min(tbY, window.innerHeight - 40)}px`;
    toolbar.style.display = "flex";
  }

  canvas.addEventListener("mousedown", (e: MouseEvent) => {
    if (hasSelection) {
      hasSelection = false;
      toolbar.style.display = "none";
    }
    isDragging = true;
    rect.startX = e.clientX; rect.startY = e.clientY;
    rect.endX = e.clientX; rect.endY = e.clientY;
  });

  canvas.addEventListener("mousemove", (e: MouseEvent) => {
    if (!isDragging) return;
    rect.endX = e.clientX; rect.endY = e.clientY;
    drawOverlay();
  });

  canvas.addEventListener("mouseup", () => {
    isDragging = false;
    selX = Math.min(rect.startX, rect.endX);
    selY = Math.min(rect.startY, rect.endY);
    selW = Math.abs(rect.endX - rect.startX);
    selH = Math.abs(rect.endY - rect.startY);

    if (selW > 5 && selH > 5) {
      hasSelection = true;
      drawOverlay();
      showToolbar();
    }
  });

  toolbar.querySelector("#btn-record")!.addEventListener("click", async () => {
    if (hasSelection) {
      await startGifRecording(selX, selY, selW, selH);
    }
    const win = getCurrentWindow();
    await win.close();
  });

  toolbar.querySelector("#btn-exit")!.addEventListener("click", async () => {
    const win = getCurrentWindow();
    await win.close();
  });

  document.addEventListener("keydown", async (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      const win = getCurrentWindow();
      await win.close();
    }
    if (e.key === "Enter" && hasSelection) {
      await startGifRecording(selX, selY, selW, selH);
      const win = getCurrentWindow();
      await win.close();
    }
  });

  canvas.setAttribute("tabindex", "0");
  canvas.focus();
  drawOverlay();
}
