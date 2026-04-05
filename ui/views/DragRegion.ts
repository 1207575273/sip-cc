import { takeSnap } from "../bridge/snapBridge";
import { startGifRecording } from "../bridge/gifBridge";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Mode = "snap" | "gif";

export function mountDragRegion(container: HTMLElement, mode: Mode): void {
  const wrapper = document.createElement("div");
  wrapper.id = "drag-region";
  wrapper.innerHTML = `
    <div class="drag-handle drag-handle-tl"></div>
    <div class="drag-handle drag-handle-tr"></div>
    <div class="drag-handle drag-handle-bl"></div>
    <div class="drag-handle drag-handle-br"></div>
    <div class="drag-info"></div>
    <div class="drag-confirm">按 Enter 确认</div>
  `;
  container.appendChild(wrapper);

  let regionX = Math.round(window.innerWidth / 4);
  let regionY = Math.round(window.innerHeight / 4);
  let regionW = Math.round(window.innerWidth / 2);
  let regionH = Math.round(window.innerHeight / 2);

  function updatePosition(): void {
    wrapper.style.left = `${regionX}px`;
    wrapper.style.top = `${regionY}px`;
    wrapper.style.width = `${regionW}px`;
    wrapper.style.height = `${regionH}px`;
    const info = wrapper.querySelector(".drag-info") as HTMLElement;
    info.textContent = `${regionW} × ${regionH}`;
  }

  updatePosition();

  let isDragging = false;
  let dragOffsetX = 0;
  let dragOffsetY = 0;

  wrapper.addEventListener("mousedown", (e: MouseEvent) => {
    if ((e.target as HTMLElement).classList.contains("drag-handle")) return;
    isDragging = true;
    dragOffsetX = e.clientX - regionX;
    dragOffsetY = e.clientY - regionY;
  });

  document.addEventListener("mousemove", (e: MouseEvent) => {
    if (!isDragging) return;
    regionX = e.clientX - dragOffsetX;
    regionY = e.clientY - dragOffsetY;
    updatePosition();
  });

  document.addEventListener("mouseup", () => { isDragging = false; });

  document.addEventListener("keydown", async (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      if (mode === "snap") {
        await takeSnap(regionX, regionY, regionW, regionH);
      } else {
        await startGifRecording(regionX, regionY, regionW, regionH);
      }
      const win = getCurrentWindow();
      await win.close();
    }
    if (e.key === "Escape") {
      const win = getCurrentWindow();
      await win.close();
    }
  });
}
