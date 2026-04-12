import { invoke } from "@tauri-apps/api/core";

// 选区遮罩常量
const MIN_SELECTION_SIZE = 5;
const OVERLAY_FONT = "14px monospace";
const OVERLAY_BORDER_WIDTH = 2;
const OVERLAY_MASK_COLOR = "rgba(0, 0, 0, 0.4)";

interface Rect {
  startX: number; startY: number; endX: number; endY: number;
}

export interface OverlayConfig {
  color: string;
  buttonText: string;
  toolbarWidth: number;
  onConfirm: (x: number, y: number, w: number, h: number) => Promise<unknown>;
  renderExtra?: (slot: HTMLElement) => void;
}

async function hideOverlay(): Promise<void> {
  try { await invoke("close_overlay"); } catch (_) {}
}

export function mountSelectionOverlay(container: HTMLElement, config: OverlayConfig): () => void {
  const canvas = document.createElement("canvas");
  canvas.id = "selection-canvas";
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
    <div class="toolbar-extra" id="toolbar-extra"></div>
    <button class="toolbar-btn toolbar-btn-save" id="btn-confirm">${config.buttonText}</button>
    <button class="toolbar-btn toolbar-btn-exit" id="btn-exit">退出</button>
  `;
  container.appendChild(toolbar);

  if (config.renderExtra) {
    config.renderExtra(toolbar.querySelector("#toolbar-extra")!);
  }

  function drawOverlay(): void {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = OVERLAY_MASK_COLOR;
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    const x = hasSelection ? selX : Math.min(rect.startX, rect.endX);
    const y = hasSelection ? selY : Math.min(rect.startY, rect.endY);
    const w = hasSelection ? selW : Math.abs(rect.endX - rect.startX);
    const h = hasSelection ? selH : Math.abs(rect.endY - rect.startY);

    if ((isDragging || hasSelection) && w > 0 && h > 0) {
      ctx.clearRect(x, y, w, h);
      ctx.strokeStyle = config.color;
      ctx.lineWidth = OVERLAY_BORDER_WIDTH;
      ctx.strokeRect(x, y, w, h);
      ctx.fillStyle = config.color;
      ctx.font = OVERLAY_FONT;
      ctx.fillText(`${w} × ${h}`, x, y > 20 ? y - 6 : y + h + 16);
    }
  }

  function showToolbar(): void {
    const tbX = selX + selW - config.toolbarWidth;
    const tbY = selY + selH + 8;
    toolbar.style.left = `${Math.max(0, tbX)}px`;
    toolbar.style.top = `${Math.min(tbY, window.innerHeight - 40)}px`;
    toolbar.style.display = "flex";
  }

  async function doConfirm(): Promise<void> {
    if (!hasSelection) return;
    try { await config.onConfirm(selX, selY, selW, selH); } catch (e) {
      console.error("操作失败:", e);
      await hideOverlay();
    }
  }

  const onMouseDown = (e: MouseEvent) => {
    if (hasSelection) { hasSelection = false; toolbar.style.display = "none"; }
    isDragging = true;
    rect.startX = e.clientX; rect.startY = e.clientY;
    rect.endX = e.clientX; rect.endY = e.clientY;
  };
  const onMouseMove = (e: MouseEvent) => {
    if (!isDragging) return;
    rect.endX = e.clientX; rect.endY = e.clientY;
    drawOverlay();
  };
  const onMouseUp = () => {
    isDragging = false;
    selX = Math.min(rect.startX, rect.endX);
    selY = Math.min(rect.startY, rect.endY);
    selW = Math.abs(rect.endX - rect.startX);
    selH = Math.abs(rect.endY - rect.startY);
    if (selW > MIN_SELECTION_SIZE && selH > MIN_SELECTION_SIZE) {
      hasSelection = true; drawOverlay(); showToolbar();
    }
  };
  const onKeyDown = async (e: KeyboardEvent) => {
    if (e.key === "Escape") await hideOverlay();
    if (e.key === "Enter" && hasSelection) await doConfirm();
  };

  canvas.addEventListener("mousedown", onMouseDown);
  canvas.addEventListener("mousemove", onMouseMove);
  canvas.addEventListener("mouseup", onMouseUp);
  document.addEventListener("keydown", onKeyDown);

  toolbar.querySelector("#btn-confirm")!.addEventListener("click", () => doConfirm());
  toolbar.querySelector("#btn-exit")!.addEventListener("click", () => hideOverlay());

  canvas.setAttribute("tabindex", "0");
  canvas.focus();
  drawOverlay();

  return () => {
    canvas.removeEventListener("mousedown", onMouseDown);
    canvas.removeEventListener("mousemove", onMouseMove);
    canvas.removeEventListener("mouseup", onMouseUp);
    document.removeEventListener("keydown", onKeyDown);
  };
}
