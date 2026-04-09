import { invoke } from "@tauri-apps/api/core";

/** 系统保留键：橙色警告但不阻止 */
const SYSTEM_RESERVED = [
  "Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+A",
  "Ctrl+S", "Ctrl+W", "Ctrl+Q", "Alt+F4",
];

/** 平台默认快捷键 */
const PLATFORM_DEFAULTS = {
  snap: navigator.platform.startsWith("Mac") ? "Ctrl+Shift+1" : "F1",
  gif: navigator.platform.startsWith("Mac") ? "Ctrl+Shift+3" : "F3",
  force_quit: "Ctrl+C,Ctrl+C",
};

interface HotkeyConfig {
  snap: string;
  gif: string;
  force_quit: string;
}

type HotkeyField = keyof HotkeyConfig;

const FIELD_LABELS: Record<HotkeyField, string> = {
  snap: "截屏",
  gif: "录制 GIF",
  force_quit: "强制退出",
};

const FIELD_ORDER: HotkeyField[] = ["snap", "gif", "force_quit"];

export function mountHotkeySettings(container: HTMLElement): void {
  // 当前编辑态的值（未保存）
  let draft: HotkeyConfig = { ...PLATFORM_DEFAULTS };
  // 当前正在监听按键的字段
  let listeningField: HotkeyField | null = null;
  // 是否录制中（录制中禁止修改）
  let isRecording = false;

  container.innerHTML = `
    <div class="hotkey-window">
      <h2 class="hotkey-title">快捷键设置</h2>
      <div class="hotkey-list" id="hotkey-list"></div>
      <div class="hotkey-tip" id="hotkey-tip"></div>
      <div class="hotkey-actions">
        <button class="hotkey-btn hotkey-btn-default" id="btn-reset">恢复默认</button>
        <button class="hotkey-btn hotkey-btn-save" id="btn-save">保存</button>
      </div>
    </div>
  `;

  const listEl = container.querySelector<HTMLElement>("#hotkey-list")!;
  const tipEl = container.querySelector<HTMLElement>("#hotkey-tip")!;
  const btnReset = container.querySelector<HTMLButtonElement>("#btn-reset")!;
  const btnSave = container.querySelector<HTMLButtonElement>("#btn-save")!;

  // --- 渲染列表 ---
  function renderList(): void {
    listEl.innerHTML = "";
    for (const field of FIELD_ORDER) {
      const row = document.createElement("div");
      row.className = "hotkey-row";
      if (listeningField === field) {
        row.classList.add("hotkey-row--listening");
      }

      const label = document.createElement("span");
      label.className = "hotkey-label";
      label.textContent = FIELD_LABELS[field];

      const valueBox = document.createElement("span");
      valueBox.className = "hotkey-value";
      valueBox.textContent = listeningField === field ? "按下新快捷键..." : draft[field];

      const btn = document.createElement("button");
      btn.className = "hotkey-btn hotkey-btn-edit";
      btn.textContent = listeningField === field ? "取消" : "修改";
      btn.disabled = isRecording;
      btn.addEventListener("click", () => {
        if (listeningField === field) {
          // 取消监听
          listeningField = null;
        } else {
          listeningField = field;
        }
        renderList();
        updateTip();
      });

      row.appendChild(label);
      row.appendChild(valueBox);
      row.appendChild(btn);
      listEl.appendChild(row);
    }
  }

  // --- 提示区域 ---
  function updateTip(): void {
    // 冲突检测
    const conflict = detectConflict();
    if (conflict) {
      tipEl.textContent = conflict;
      tipEl.className = "hotkey-tip hotkey-tip--error";
      btnSave.disabled = true;
      return;
    }
    // 系统保留键警告
    const reserved = detectReserved();
    if (reserved) {
      tipEl.textContent = reserved;
      tipEl.className = "hotkey-tip hotkey-tip--warn";
      btnSave.disabled = false;
      return;
    }
    if (listeningField) {
      tipEl.textContent = "请按下快捷键组合，Esc 取消";
      tipEl.className = "hotkey-tip hotkey-tip--info";
    } else {
      tipEl.textContent = "";
      tipEl.className = "hotkey-tip";
    }
    btnSave.disabled = false;
  }

  function detectConflict(): string | null {
    const values = FIELD_ORDER.map((f) => draft[f]).filter(Boolean);
    const seen = new Map<string, string>();
    for (const field of FIELD_ORDER) {
      const val = draft[field];
      if (!val) continue;
      const existing = seen.get(val);
      if (existing) {
        return `冲突：「${FIELD_LABELS[field]}」和「${FIELD_LABELS[existing as HotkeyField]}」绑定了相同的快捷键 ${val}`;
      }
      seen.set(val, field);
    }
    return null;
  }

  function detectReserved(): string | null {
    for (const field of FIELD_ORDER) {
      const val = draft[field];
      if (SYSTEM_RESERVED.includes(val)) {
        return `注意：「${FIELD_LABELS[field]}」使用了系统保留键 ${val}，可能与系统功能冲突`;
      }
    }
    return null;
  }

  // --- 按键捕获 ---
  function onKeydown(e: KeyboardEvent): void {
    if (!listeningField) return;

    e.preventDefault();
    e.stopPropagation();

    // Esc 取消
    if (e.key === "Escape") {
      listeningField = null;
      renderList();
      updateTip();
      return;
    }

    // 忽略纯修饰键
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");
    parts.push(codeToKeyName(e.code));

    draft[listeningField] = parts.join("+");
    listeningField = null;
    renderList();
    updateTip();
  }

  document.addEventListener("keydown", onKeydown, true);

  // --- 按钮事件 ---
  btnReset.addEventListener("click", () => {
    draft = { ...PLATFORM_DEFAULTS };
    listeningField = null;
    renderList();
    updateTip();
  });

  btnSave.addEventListener("click", async () => {
    btnSave.disabled = true;
    btnSave.textContent = "保存中...";
    try {
      await invoke("set_hotkeys", { hotkeys: draft });
      tipEl.textContent = "保存成功！";
      tipEl.className = "hotkey-tip hotkey-tip--info";
      // 延迟关窗口，让用户看到反馈
      setTimeout(async () => {
        try {
          const { getCurrentWindow } = await import("@tauri-apps/api/window");
          await getCurrentWindow().close();
        } catch (_) {
          // 窗口关闭失败不影响保存结果
        }
      }, 500);
    } catch (err) {
      tipEl.textContent = `保存失败：${err}`;
      tipEl.className = "hotkey-tip hotkey-tip--error";
      btnSave.disabled = false;
      btnSave.textContent = "保存";
    }
  });

  // --- 初始化加载 ---
  async function init(): Promise<void> {
    try {
      const [hotkeys, recording] = await Promise.all([
        invoke<HotkeyConfig>("get_hotkeys"),
        invoke<boolean>("get_recording_status"),
      ]);
      draft = { ...hotkeys };
      isRecording = recording;
    } catch (err) {
      tipEl.textContent = `加载失败：${err}`;
      tipEl.className = "hotkey-tip hotkey-tip--error";
    }
    renderList();
    updateTip();
    if (isRecording) {
      tipEl.textContent = "录制进行中，无法修改快捷键";
      tipEl.className = "hotkey-tip hotkey-tip--warn";
    }
  }

  init();
}

function codeToKeyName(code: string): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (code.startsWith("Numpad")) return code.slice(6);
  return code;
}
