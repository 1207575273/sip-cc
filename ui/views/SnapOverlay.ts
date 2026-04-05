import { takeSnap } from "../bridge/snapBridge";
import { invoke } from "@tauri-apps/api/core";

interface Rect {
  startX: number; startY: number; endX: number; endY: number;
}

/** 通过 Rust 端强制关闭所有 overlay 窗口 */
async function forceCloseOverlay(): Promise<void> {
  try {
    await invoke("close_overlay");
  } catch (_) {
    // 如果命令也失败，尝试前端方式
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      await win.setAlwaysOnTop(false);
      await win.setFullscreen(false);
      await win.close();
    } catch (__) {
      document.body.style.display = "none";
    }
  }
}

export function mountSnapOverlay(container: HTMLElement): void {
  const canvas = document.createElement("canvas");
  canvas.id = "snap-canvas";
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
    <button class="toolbar-btn toolbar-btn-save" id="btn-save">保存</button>
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
      ctx.strokeStyle = "#00aaff";
      ctx.lineWidth = 2;
      ctx.strokeRect(x, y, w, h);

      ctx.fillStyle = "#00aaff";
      ctx.font = "14px monospace";
      ctx.fillText(`${w} × ${h}`, x, y > 20 ? y - 6 : y + h + 16);
    }
  }

  function showToolbar(): void {
    const tbX = selX + selW - 130;
    const tbY = selY + selH + 8;
    toolbar.style.left = `${Math.max(0, tbX)}px`;
    toolbar.style.top = `${Math.min(tbY, window.innerHeight - 40)}px`;
    toolbar.style.display = "flex";
  }

  // 保存：调用 Rust snap_region（Rust 端会先关窗口再截屏）
  async function doSave(): Promise<void> {
    if (!hasSelection) return;
    try {
      await takeSnap(selX, selY, selW, selH);
    } catch (e) {
      console.error("截屏失败:", e);
      // 即使失败也要关窗口
      await forceCloseOverlay();
    }
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

  toolbar.querySelector("#btn-save")!.addEventListener("click", () => doSave());
  toolbar.querySelector("#btn-exit")!.addEventListener("click", () => forceCloseOverlay());

  document.addEventListener("keydown", async (e: KeyboardEvent) => {
    if (e.key === "Escape") await forceCloseOverlay();
    if (e.key === "Enter" && hasSelection) await doSave();
  });

  canvas.setAttribute("tabindex", "0");
  canvas.focus();
  drawOverlay();
}
