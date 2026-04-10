import { listen } from "@tauri-apps/api/event";
import { downloadFfmpeg } from "../bridge/videoBridge";
import { invoke } from "@tauri-apps/api/core";

export function mountFfmpegDownload(container: HTMLElement): void {
  container.innerHTML = `
    <div class="ffmpeg-download">
      <div class="ffmpeg-icon">&#127916;</div>
      <h2>视频录制需要 FFmpeg 支持</h2>
      <p class="ffmpeg-desc">
        FFmpeg 是开源视频编码工具，首次使用需下载（约 30MB），<br/>
        下载后自动配置，无需手动安装。
      </p>
      <div class="ffmpeg-progress" style="display: none;">
        <div class="ffmpeg-progress-bar">
          <div class="ffmpeg-progress-fill" id="progress-fill"></div>
        </div>
        <span class="ffmpeg-progress-text" id="progress-text">0%</span>
      </div>
      <div class="ffmpeg-actions" id="actions">
        <button class="ffmpeg-btn ffmpeg-btn-primary" id="btn-download">下载并启用</button>
        <button class="ffmpeg-btn ffmpeg-btn-secondary" id="btn-cancel">取消</button>
      </div>
      <p class="ffmpeg-error" id="error-msg" style="display: none;"></p>
    </div>
  `;

  const progressContainer = container.querySelector(".ffmpeg-progress") as HTMLElement;
  const progressFill = container.querySelector("#progress-fill") as HTMLElement;
  const progressText = container.querySelector("#progress-text") as HTMLElement;
  const actionsEl = container.querySelector("#actions") as HTMLElement;
  const errorMsg = container.querySelector("#error-msg") as HTMLElement;
  const downloadBtn = container.querySelector("#btn-download") as HTMLButtonElement;
  const cancelBtn = container.querySelector("#btn-cancel") as HTMLButtonElement;

  listen<[number, number]>("ffmpeg-download-progress", (event) => {
    const [downloaded, total] = event.payload;
    if (total > 0) {
      const percent = Math.round((downloaded / total) * 100);
      progressFill.style.width = `${percent}%`;
      progressText.textContent = `${percent}% (${(downloaded / 1024 / 1024).toFixed(1)}MB)`;
    }
  });

  downloadBtn.addEventListener("click", async () => {
    actionsEl.style.display = "none";
    progressContainer.style.display = "flex";
    errorMsg.style.display = "none";

    try {
      await downloadFfmpeg();
      await invoke("close_window", { label: "ffmpeg-download" });
    } catch (e) {
      progressContainer.style.display = "none";
      actionsEl.style.display = "flex";
      errorMsg.style.display = "block";
      errorMsg.textContent = `下载失败: ${e}`;
    }
  });

  cancelBtn.addEventListener("click", async () => {
    await invoke("close_window", { label: "ffmpeg-download" });
  });
}
