import { invoke } from "@tauri-apps/api/core";

export function mountAbout(container: HTMLElement): void {
  container.innerHTML = `
    <div class="about-window">
      <div class="about-icon">📷</div>
      <h1 class="about-title">sip-cc</h1>
      <p class="about-version">v${__APP_VERSION__}</p>
      <p class="about-desc">轻量跨平台截屏 + GIF 录制工具</p>
      <div class="about-divider"></div>
      <p class="about-author">作者：<strong>codeYang</strong></p>
      <a class="about-link" id="github-link" href="#">
        github.com/1207575273/sip-cc
      </a>
      <p class="about-star">⭐ 欢迎 Star 支持！</p>
      <div class="about-divider"></div>
      <button class="about-btn" id="btn-close">关闭</button>
    </div>
  `;

  // 点击 GitHub 链接用系统浏览器打开
  container.querySelector("#github-link")!.addEventListener("click", async (e) => {
    e.preventDefault();
    await invoke("open_url", { url: "https://github.com/1207575273/sip-cc" });
  });

  // 关闭按钮
  container.querySelector("#btn-close")!.addEventListener("click", async () => {
    try {
      await invoke("close_window", { label: "about" });
    } catch (_) {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().close();
      } catch (__) {}
    }
  });
}

// vite 注入的版本号
declare const __APP_VERSION__: string;
