import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  callLongSnapCancel,
  callLongSnapFinish,
  callLongSnapStart,
  callSetOverlayCursorPassthrough,
} from "../shared/invoke";

// 选区遮罩常量
const MIN_SELECTION_SIZE = 5;
const OVERLAY_FONT = "14px monospace";
const OVERLAY_BORDER_WIDTH = 2;
const OVERLAY_MASK_COLOR = "rgba(0, 0, 0, 0.4)";
const LONG_SNAP_TOOLBAR_W = 340;

interface Rect {
  startX: number; startY: number; endX: number; endY: number;
}

export interface OverlayConfig {
  color: string;
  buttonText: string;
  toolbarWidth: number;
  onConfirm: (x: number, y: number, w: number, h: number) => Promise<unknown>;
  /** Windows：长截图入口（采集层取景框 + 物理层在框内滚动） */
  longSnapLabel?: string;
  longSnapSupported?: boolean;
  renderExtra?: (slot: HTMLElement) => void;
}

function monitorIndexFromUrl(): number | undefined {
  const m = new URLSearchParams(window.location.search).get("monitor");
  if (m === null) return undefined;
  const n = parseInt(m, 10);
  return Number.isNaN(n) ? undefined : n;
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

  /** 采集层：取景框 + 四边遮罩；滚轮须配合 Rust set_ignore_cursor_events 穿透到下层 */
  let longSnapActive = false;
  let longSnapLayer: HTMLDivElement | null = null;
  let unlistenProgress: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenCancelled: UnlistenFn | null = null;

  const toolbar = document.createElement("div");
  toolbar.className = "selection-toolbar";
  toolbar.style.display = "none";
  toolbar.innerHTML = `
    <div id="toolbar-normal" class="toolbar-mode-row">
      <div class="toolbar-extra" id="toolbar-extra"></div>
      <button type="button" class="toolbar-btn toolbar-btn-long" id="btn-long-snap" style="display:none" title="长截图（仅 Windows）">长截图</button>
      <button type="button" class="toolbar-btn toolbar-btn-save" id="btn-confirm">${config.buttonText}</button>
      <button type="button" class="toolbar-btn toolbar-btn-exit" id="btn-exit">退出</button>
    </div>
    <div id="toolbar-longsnap" class="toolbar-mode-row" style="display:none">
      <span class="long-snap-inline-hint" id="ls-inline-hint">已 0 帧 · 滚轮采样（有变化才加帧）· 完成/取消见浮动条</span>
    </div>
  `;
  container.appendChild(toolbar);

  const toolbarNormal = toolbar.querySelector("#toolbar-normal") as HTMLElement;
  const toolbarLongSnap = toolbar.querySelector("#toolbar-longsnap") as HTMLElement;
  const lsHint = toolbar.querySelector("#ls-inline-hint") as HTMLSpanElement;

  if (config.renderExtra) {
    config.renderExtra(toolbar.querySelector("#toolbar-extra")!);
  }

  const btnLong = toolbar.querySelector("#btn-long-snap") as HTMLButtonElement;
  if (config.longSnapSupported && config.longSnapLabel) {
    btnLong.textContent = config.longSnapLabel;
    btnLong.style.display = "inline-flex";
    btnLong.title =
      "进入长截图：主窗口穿透后滚轮由后端监听；滚动后画面变化才加帧。完成/取消在选区旁浮动条。";
  }

  function showLsToast(message: string, isError: boolean): void {
    const t = document.createElement("div");
    t.className = "long-snap-toast";
    if (isError) t.classList.add("long-snap-toast--error");
    t.textContent = message;
    document.body.appendChild(t);
    setTimeout(() => t.remove(), 4200);
  }

  function updateLsHint(current: number): void {
    lsHint.textContent =
      `已 ${current} 帧 · 滚轮+画面变化采样 · 完成/取消见浮动条`;
  }

  function teardownLongSnapSync(): void {
    longSnapActive = false;
    void callSetOverlayCursorPassthrough(false).catch(() => {});
    unlistenProgress?.();
    unlistenProgress = null;
    unlistenError?.();
    unlistenError = null;
    unlistenDone?.();
    unlistenDone = null;
    unlistenCancelled?.();
    unlistenCancelled = null;
    longSnapLayer?.remove();
    longSnapLayer = null;
    canvas.style.visibility = "";
    toolbarNormal.style.display = "flex";
    toolbarLongSnap.style.display = "none";
  }

  async function exitLongSnapCaptureMode(): Promise<void> {
    teardownLongSnapSync();
    drawOverlay();
    showToolbar();
  }

  async function enterLongSnapCaptureMode(): Promise<void> {
    longSnapActive = true;
    try {
      await callSetOverlayCursorPassthrough(true);
    } catch (e) {
      console.warn("set_overlay_cursor_passthrough:", e);
    }
    canvas.style.visibility = "hidden";
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    longSnapLayer = document.createElement("div");
    longSnapLayer.className = "long-snap-capture-layer";
    longSnapLayer.style.cssText =
      "position:fixed;inset:0;pointer-events:none;z-index:9998;";

    const x = selX, y = selY, w = selW, h = selH;
    const W = window.innerWidth;
    const H = window.innerHeight;

    const mk = (s: string) => {
      const d = document.createElement("div");
      d.setAttribute("style", s);
      return d;
    };
    longSnapLayer.appendChild(
      mk(`position:absolute;left:0;top:0;width:100%;height:${y}px;background:${OVERLAY_MASK_COLOR}`),
    );
    longSnapLayer.appendChild(
      mk(`position:absolute;left:0;top:${y}px;width:${x}px;height:${h}px;background:${OVERLAY_MASK_COLOR}`),
    );
    longSnapLayer.appendChild(
      mk(`position:absolute;left:${x + w}px;top:${y}px;width:${W - x - w}px;height:${h}px;background:${OVERLAY_MASK_COLOR}`),
    );
    longSnapLayer.appendChild(
      mk(`position:absolute;left:0;top:${y + h}px;width:100%;height:${H - y - h}px;background:${OVERLAY_MASK_COLOR}`),
    );
    const frame = document.createElement("div");
    frame.style.cssText =
      `position:absolute;left:${x}px;top:${y}px;width:${w}px;height:${h}px;` +
      `border:${OVERLAY_BORDER_WIDTH}px solid ${config.color};box-sizing:border-box;pointer-events:none`;
    longSnapLayer.appendChild(frame);

    container.insertBefore(longSnapLayer, toolbar);

    toolbarNormal.style.display = "none";
    toolbarLongSnap.style.display = "flex";
    updateLsHint(0);
    showToolbar();

    unlistenProgress = await listen<{ current: number; max: number }>(
      "long-snap-progress",
      (e) => updateLsHint(e.payload.current),
    );
    unlistenError = await listen<{ message: string }>("long-snap-error", (e) => {
      showLsToast(e.payload.message, true);
    });
    unlistenDone = await listen("long-snap-done", async () => {
      if (!longSnapActive) return;
      try {
        await exitLongSnapCaptureMode();
        await hideOverlay();
      } catch (_) {}
    });
    unlistenCancelled = await listen("long-snap-cancelled", async () => {
      if (!longSnapActive) return;
      try {
        await exitLongSnapCaptureMode();
        await hideOverlay();
      } catch (_) {}
    });
  }

  /** 主窗口内 Enter：仅发起 finish，UI 收尾由 long-snap-done 事件统一处理 */
  async function doLongSnapFinish(): Promise<void> {
    try {
      await callLongSnapFinish();
    } catch (e) {
      showLsToast(String(e), true);
    }
  }

  async function doLongSnapCancel(): Promise<void> {
    try {
      await callLongSnapCancel();
    } catch (e) {
      showLsToast(String(e), true);
      await exitLongSnapCaptureMode();
      await hideOverlay();
    }
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
    const tw = longSnapActive ? LONG_SNAP_TOOLBAR_W : config.toolbarWidth;
    const tbX = selX + selW - tw;
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

  async function doLongSnap(): Promise<void> {
    if (!hasSelection || !config.longSnapSupported) return;
    const mon = monitorIndexFromUrl();
    const region = {
      x: selX,
      y: selY,
      width: selW,
      height: selH,
      ...(mon !== undefined ? { monitor_index: mon } : {}),
    };
    try {
      toolbar.querySelectorAll("#toolbar-normal .toolbar-btn").forEach((b) => {
        (b as HTMLButtonElement).disabled = true;
      });
      await callLongSnapStart(region);
      await enterLongSnapCaptureMode();
    } catch (e) {
      console.error("长截图失败:", e);
      await hideOverlay();
    } finally {
      toolbar.querySelectorAll("#toolbar-normal .toolbar-btn").forEach((b) => {
        (b as HTMLButtonElement).disabled = false;
      });
    }
  }

  const onMouseDown = (e: MouseEvent) => {
    if (longSnapActive) return;
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
    if (longSnapActive) return;
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
    if (longSnapActive) {
      if (e.key === "Enter") {
        e.preventDefault();
        await doLongSnapFinish();
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        await doLongSnapCancel();
        return;
      }
      return;
    }
    if (e.key === "Escape") await hideOverlay();
    if (e.key === "Enter" && hasSelection) await doConfirm();
  };

  canvas.addEventListener("mousedown", onMouseDown);
  canvas.addEventListener("mousemove", onMouseMove);
  canvas.addEventListener("mouseup", onMouseUp);
  document.addEventListener("keydown", onKeyDown);

  toolbar.querySelector("#btn-confirm")!.addEventListener("click", () => doConfirm());
  toolbar.querySelector("#btn-exit")!.addEventListener("click", () => hideOverlay());
  if (config.longSnapSupported) {
    btnLong.addEventListener("click", () => doLongSnap());
  }
  canvas.setAttribute("tabindex", "0");
  canvas.focus();
  drawOverlay();

  return () => {
    canvas.removeEventListener("mousedown", onMouseDown);
    canvas.removeEventListener("mousemove", onMouseMove);
    canvas.removeEventListener("mouseup", onMouseUp);
    document.removeEventListener("keydown", onKeyDown);
    teardownLongSnapSync();
  };
}
