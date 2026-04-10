import { mountSnapOverlay } from "./views/SnapOverlay";
import { mountGifOverlay } from "./views/GifOverlay";
import { mountVideoOverlay } from "./views/VideoOverlay";
import { mountFfmpegDownload } from "./views/FfmpegDownload";
import { mountRecordBar } from "./views/RecordBar";
import { mountRecordRegion } from "./views/RecordRegion";
import { mountAbout } from "./views/About";
import { mountHotkeySettings } from "./views/HotkeySettings";
import { listen } from "@tauri-apps/api/event";

const container = document.getElementById("app")!;
const params = new URLSearchParams(window.location.search);
const view = params.get("view");

if (view === "record-bar") {
  mountRecordBar(container);
} else if (view === "record-region") {
  mountRecordRegion(container);
} else if (view === "ffmpeg-download") {
  mountFfmpegDownload(container);
} else if (view === "about") {
  mountAbout(container);
} else if (view === "hotkey-settings") {
  mountHotkeySettings(container);
} else {
  setupOverlay();
}

/** 等待窗口尺寸就绪（hide→show 后需要等一下） */
function waitForWindowReady(): Promise<void> {
  return new Promise((resolve) => {
    function check() {
      if (window.innerWidth > 100 && window.innerHeight > 100) {
        resolve();
      } else {
        requestAnimationFrame(check);
      }
    }
    // 给一帧的时间让窗口完全展开
    requestAnimationFrame(check);
  });
}

function setupOverlay(): void {
  let cleanup: (() => void) | null = null;

  async function switchMode(mode: string): Promise<void> {
    if (cleanup) { cleanup(); cleanup = null; }
    container.innerHTML = "";

    // 等窗口完全展开再挂载（解决第二次打开 canvas 尺寸为 0 的问题）
    await waitForWindowReady();

    if (mode === "snap-overlay") {
      cleanup = mountSnapOverlay(container);
    } else if (mode === "gif-overlay") {
      cleanup = mountGifOverlay(container);
    } else if (mode === "video-overlay") {
      cleanup = mountVideoOverlay(container);
    }
  }

  listen("overlay-mode", (event) => {
    switchMode(event.payload as string);
  });

  // 初始加载
  if (view === "snap-overlay") {
    cleanup = mountSnapOverlay(container);
  } else if (view === "gif-overlay") {
    cleanup = mountGifOverlay(container);
  } else if (view === "video-overlay") {
    cleanup = mountVideoOverlay(container);
  }
}
