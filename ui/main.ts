import { mountSnapOverlay } from "./views/SnapOverlay";
import { mountGifOverlay } from "./views/GifOverlay";
import { mountRecordBar } from "./views/RecordBar";
import { mountRecordRegion } from "./views/RecordRegion";
import { listen } from "@tauri-apps/api/event";

const container = document.getElementById("app")!;
const params = new URLSearchParams(window.location.search);
const view = params.get("view");

if (view === "record-bar") {
  mountRecordBar(container);
} else if (view === "record-region") {
  mountRecordRegion(container);
} else {
  setupOverlay();
}

function setupOverlay(): void {
  let cleanup: (() => void) | null = null;

  function switchMode(mode: string): void {
    // 先清理旧的事件监听器
    if (cleanup) { cleanup(); cleanup = null; }
    // 清空 DOM
    container.innerHTML = "";

    if (mode === "snap-overlay") {
      cleanup = mountSnapOverlay(container);
    } else if (mode === "gif-overlay") {
      cleanup = mountGifOverlay(container);
    }
  }

  // 监听 Rust 端发来的模式切换事件
  listen("overlay-mode", (event) => {
    switchMode(event.payload as string);
  });

  // 初始加载
  if (view === "snap-overlay") {
    cleanup = mountSnapOverlay(container);
  } else if (view === "gif-overlay") {
    cleanup = mountGifOverlay(container);
  }
}
