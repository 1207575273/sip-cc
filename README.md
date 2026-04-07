# sip-cc

轻量跨平台截屏 + GIF 录制工具。

> **命名由来**：**sip** 致敬 **Snipaste**，**cc** = **c**ode**Y**ang + **C**hina，中国开发者 codeYang 出品。

## 为什么做这个

Snipaste 好用但不支持 GIF 录制，市面上能录 GIF 的工具要么太重（Electron 套壳 100MB+），要么不跨平台。需要一个极简的工具：截屏 + 录 GIF，仅此而已。

## 功能

- **截屏**：快捷键触发，全屏遮罩框选，保存 PNG + 复制到剪贴板
- **GIF 录制**：快捷键触发，框选区域，支持暂停/继续/停止，流式编码边录边写
- **录制区域指示**：录制时红色脉冲边框标识录制范围，鼠标可穿透
- **系统托盘**：常驻后台，右键菜单操作
- **配置文件**：`~/.sip-cc/config.json`，可调帧率、时长、保存目录
- **WAL 日志**：所有操作先写日志，崩溃可追溯

## 快捷键

| 功能 | Windows / Linux | macOS |
|------|----------------|-------|
| 截屏 | `F1` | `⌘⇧1` (Cmd+Shift+1) |
| 录制 GIF | `F3` | `⌘⇧3` (Cmd+Shift+3) |
| 暂停/继续录制 | `Space` | `Space` |
| 停止录制 | `Esc` / `Enter` | `Esc` / `Enter` |
| 强制退出 | 双击 `Ctrl+C` | 双击 `⌘C` |

## 平台差异

| | Windows | macOS | Linux |
|---|---------|-------|-------|
| **安装** | 双击 `.exe` 即用，无需安装 | 双击 `.app` 运行 | 需要 `webkit2gtk` |
| **首次运行** | 无额外步骤 | 需授权**辅助功能权限**（自动弹窗引导） | 无额外步骤 |
| **截屏剪贴板** | RGBA 图像数据 | PNG 格式（兼容微信等应用） | RGBA 图像数据 |
| **GIF 剪贴板** | 文件路径文本 | 文件引用（可直接粘贴为文件） | 文件路径文本 |
| **全屏遮罩** | 系统全屏 | 窗口覆盖屏幕（避免独立 Space 黑屏） | 系统全屏 |

### macOS 辅助功能权限

macOS 上全局快捷键需要**辅助功能（Accessibility）权限**。首次启动时 sip-cc 会自动弹出系统授权引导：

> "sip-cc 想要控制此电脑"

点击「打开系统设置」→ 勾选 sip-cc → 完成。这是一次性操作，所有需要监听全局键盘的 macOS 应用（Snipaste、Alfred、Raycast 等）都需要这一步。

如果弹窗未出现，手动前往：**系统设置 → 隐私与安全性 → 辅助功能** → 添加 sip-cc。

## 技术栈

Tauri v2（Rust + WebView），前端原生 TypeScript + DOM，无框架。

| 能力 | 依赖 |
|------|------|
| 屏幕捕获 | xcap |
| GIF 编码 | gif crate（流式写入） |
| 全局快捷键 | rdev |
| 剪贴板 | arboard + osascript (macOS) |
| 系统托盘 | Tauri 内置 |

## 构建

```bash
npm install
npm run tauri dev    # 开发
npm run tauri build  # 打包
```

## 配置

首次启动自动生成 `~/.sip-cc/config.json`：

```json
{
  "save_dir": "desktop",
  "custom_save_dir": null,
  "gif_fps": 10,
  "gif_max_duration_secs": 180
}
```

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `save_dir` | 保存位置：`desktop` 或 `custom` | `desktop` |
| `custom_save_dir` | 自定义保存目录路径 | `null` |
| `gif_fps` | GIF 录制帧率 | `10` |
| `gif_max_duration_secs` | GIF 最大录制时长（秒） | `180`（3分钟） |

可通过托盘菜单「打开配置文件」直接编辑，也可在托盘菜单中切换保存目录。
