import { pauseGif, resumeGif, stopGif } from "../bridge/gifBridge";

export function mountRecordBar(container: HTMLElement): void {
  const bar = document.createElement("div");
  bar.id = "record-bar";
  bar.innerHTML = `
    <span class="record-indicator">\u25CF</span>
    <span class="record-status">录制中</span>
    <span class="record-timer">00:00</span>
    <span class="record-divider"></span>
    <button class="record-btn" id="btn-pause">暂停</button>
    <button class="record-btn record-btn-stop" id="btn-stop">停止</button>
  `;
  container.appendChild(bar);

  const statusEl = bar.querySelector(".record-status") as HTMLElement;
  const timerEl = bar.querySelector(".record-timer") as HTMLElement;
  const indicatorEl = bar.querySelector(".record-indicator") as HTMLElement;
  const pauseBtn = bar.querySelector("#btn-pause") as HTMLButtonElement;
  const stopBtn = bar.querySelector("#btn-stop") as HTMLButtonElement;

  let elapsed = 0;
  let isPaused = false;

  const timerHandle = window.setInterval(() => {
    if (!isPaused) {
      elapsed++;
      const mins = Math.floor(elapsed / 60).toString().padStart(2, "0");
      const secs = (elapsed % 60).toString().padStart(2, "0");
      timerEl.textContent = `${mins}:${secs}`;
    }
  }, 1000);

  pauseBtn.addEventListener("click", async () => {
    if (isPaused) {
      await resumeGif();
      isPaused = false;
      pauseBtn.textContent = "暂停";
      statusEl.textContent = "录制中";
      indicatorEl.style.color = "#ff4444";
    } else {
      await pauseGif();
      isPaused = true;
      pauseBtn.textContent = "继续";
      statusEl.textContent = "已暂停";
      indicatorEl.style.color = "#ffaa00";
    }
  });

  stopBtn.addEventListener("click", async () => {
    clearInterval(timerHandle);
    await stopGif();
    // gif_stop 会在独立线程关闭 record-bar 和 record-region 窗口
  });

  document.addEventListener("keydown", async (e: KeyboardEvent) => {
    if (e.code === "Space") { e.preventDefault(); pauseBtn.click(); }
    if (e.key === "Escape" || e.key === "Enter") { stopBtn.click(); }
  });
}
