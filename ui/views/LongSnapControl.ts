import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * 独立小窗：主 overlay 开启鼠标穿透后无法点击按钮，完成/取消在此窗口操作。
 */
export function mountLongSnapControl(container: HTMLElement): void {
  const bar = document.createElement("div");
  bar.id = "long-snap-control";
  bar.innerHTML = `
    <span class="lsc-hint">长截图</span>
    <button type="button" class="lsc-btn lsc-btn-primary" id="lsc-finish">完成</button>
    <button type="button" class="lsc-btn" id="lsc-cancel">取消</button>
  `;
  container.appendChild(bar);

  listen<{ message: string }>("long-snap-error", (e) => {
    showToast(e.payload.message, true);
  });

  function showToast(message: string, isError: boolean): void {
    const t = document.createElement("div");
    t.className = "long-snap-toast";
    if (isError) t.classList.add("long-snap-toast--error");
    t.textContent = message;
    document.body.appendChild(t);
    setTimeout(() => t.remove(), 5000);
  }

  const btnFinish = bar.querySelector("#lsc-finish") as HTMLButtonElement;
  const btnCancel = bar.querySelector("#lsc-cancel") as HTMLButtonElement;
  const finishLabel = "完成";

  async function runFinish(): Promise<void> {
    btnFinish.disabled = true;
    btnCancel.disabled = true;
    btnFinish.textContent = "拼接中…";
    try {
      await invoke<string>("long_snap_finish");
    } catch (e) {
      showToast(String(e), true);
      btnFinish.disabled = false;
      btnCancel.disabled = false;
      btnFinish.textContent = finishLabel;
    }
  }

  btnFinish.addEventListener("click", () => void runFinish());
  btnCancel.addEventListener("click", async () => {
    try {
      await invoke("long_snap_cancel");
    } catch (e) {
      showToast(String(e), true);
    }
  });

  const onKey = async (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      await runFinish();
    }
    if (e.key === "Escape") {
      e.preventDefault();
      try {
        await invoke("long_snap_cancel");
      } catch (err) {
        showToast(String(err), true);
      }
    }
  };
  document.addEventListener("keydown", onKey);
  bar.setAttribute("tabindex", "-1");
  bar.focus();
}
