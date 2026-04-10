import { listen } from "@tauri-apps/api/event";

export function mountRecordInfo(container: HTMLElement): void {
  const params = new URLSearchParams(window.location.search);
  const mode = params.get("mode") || "video";
  const w = params.get("w") || "0";
  const h = params.get("h") || "0";
  const fps = params.get("fps") || "0";
  const quality = params.get("quality") || "";

  const panel = document.createElement("div");
  panel.id = "record-info";
  panel.innerHTML = `
    <div class="ri-row ri-row-top">
      <span class="ri-dot"></span>
      <span class="ri-mode">${mode === "gif" ? "GIF" : "REC"}</span>
      ${quality ? `<span class="ri-quality">${quality}</span>` : ""}
    </div>
    <div class="ri-row">
      <span class="ri-meta">${w}×${h}</span>
      <span class="ri-meta">${fps}fps</span>
    </div>
    <div class="ri-time">00:00</div>
    <div class="ri-brand">codeyag · github.com/1207575273/sip-cc</div>
  `;
  container.appendChild(panel);

  const timeEl = panel.querySelector(".ri-time") as HTMLElement;
  const dotEl = panel.querySelector(".ri-dot") as HTMLElement;

  let elapsed = 0;
  let paused = false;

  const timer = window.setInterval(() => {
    if (paused) return;
    elapsed++;
    const mm = Math.floor(elapsed / 60).toString().padStart(2, "0");
    const ss = (elapsed % 60).toString().padStart(2, "0");
    timeEl.textContent = `${mm}:${ss}`;
  }, 1000);

  listen("recording-paused", () => {
    paused = true;
    dotEl.style.animationPlayState = "paused";
    dotEl.style.opacity = "0.4";
  });

  listen("recording-resumed", () => {
    paused = false;
    dotEl.style.animationPlayState = "running";
    dotEl.style.opacity = "";
  });

  listen("video-encoding-progress", () => {
    clearInterval(timer);
    dotEl.style.animation = "none";
    dotEl.style.background = "#4a90d9";
    panel.querySelector(".ri-mode")!.textContent = "ENC";
  });
}
