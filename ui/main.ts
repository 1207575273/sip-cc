import { mountSnapOverlay } from "./views/SnapOverlay";
import { mountGifOverlay } from "./views/GifOverlay";
import { mountRecordBar } from "./views/RecordBar";
import { mountRecordRegion } from "./views/RecordRegion";
import { listen } from "@tauri-apps/api/event";

const container = document.getElementById("app")!;
const params = new URLSearchParams(window.location.search);
const view = params.get("view");

// record-bar 和 record-region 仍用 URL 参数（独立小窗口）
if (view === "record-bar") {
  mountRecordBar(container);
} else if (view === "record-region") {
  mountRecordRegion(container);
} else {
  // overlay 窗口：监听 Rust 端发来的模式切换事件
  setupOverlay();
}

function setupOverlay(): void {
  listen("overlay-mode", (event) => {
    container.innerHTML = "";

    const mode = event.payload as string;
    if (mode === "snap-overlay") {
      mountSnapOverlay(container);
    } else if (mode === "gif-overlay") {
      mountGifOverlay(container);
    }
  });

  // 初始加载
  if (view === "snap-overlay") {
    mountSnapOverlay(container);
  } else if (view === "gif-overlay") {
    mountGifOverlay(container);
  }
}
