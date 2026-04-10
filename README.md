# sip-cc

轻量跨平台截屏 + GIF 录制 + 视频录制工具。

> **命名由来**：**sip** 致敬 **Snipaste**，**cc** = **c**ode**Y**ang + **C**hina，中国开发者 codeYang 出品。

## 为什么做这个

Snipaste 好用但不支持 GIF/视频录制，市面上能录屏的工具要么太重（Electron 套壳 100MB+），要么不跨平台。需要一个极简的工具：截屏 + 录 GIF + 录视频，仅此而已。

## 功能

### 截屏
- 快捷键触发，全屏遮罩框选，保存 PNG + 复制到剪贴板

### GIF 录制
- 快捷键触发，框选区域，支持暂停/继续/停止
- 流式编码，边录边写，内存占用低

### 视频录制
- 快捷键触发，框选区域后选择质量档位开始录制
- 三档质量预设：**流畅**（10fps）/ **标准**（15fps）/ **高清**（30fps）
- H.264 MP4 编码，后台非阻塞编码，实时进度显示
- FFmpeg 按需下载（首次录制时自动下载，Windows .7z ~32MB）

### 录制信息叠加
- 录制时右上角显示半透明信息面板，自动录入画面
- 显示内容：录制状态（REC/GIF/ENC）、质量档位、分辨率、帧率、实时计时
- 暂停时计时器同步暂停，编码阶段自动切换为 ENC 状态

### 通用能力
- **录制区域指示**：录制时红色脉冲边框标识录制范围，鼠标可穿透
- **快捷键自定义**：托盘菜单打开设置窗口，可视化修改所有快捷键
- **系统托盘**：常驻后台，右键菜单操作
- **配置文件**：`~/.sip-cc/config.json`，可调帧率、时长、保存目录等
- **WAL 日志**：所有操作先写日志，崩溃可追溯

## 快捷键

| 功能 | Windows / Linux | macOS |
|------|----------------|-------|
| 截屏 | `F1` | `⌘⇧1` (Cmd+Shift+1) |
| 录制 GIF | `F3` | `⌘⇧3` (Cmd+Shift+3) |
| 录制视频 | `F5` | `⌘⇧5` (Cmd+Shift+5) |
| 暂停/继续录制 | `Space` | `Space` |
| 停止录制 | `Esc` / `Enter` | `Esc` / `Enter` |
| 强制退出 | 双击 `Ctrl+C` | 双击 `⌘C` |

> 所有快捷键均可通过托盘菜单「快捷键设置」自定义修改。

## 平台差异

| | Windows | macOS | Linux |
|---|---------|-------|-------|
| **安装** | 双击 `.exe` 即用，无需安装 | 双击 `.app` 运行 | 需要 `webkit2gtk` |
| **首次运行** | 无额外步骤 | 需授权**辅助功能权限**（自动弹窗引导） | 无额外步骤 |
| **截屏剪贴板** | RGBA 图像数据 | PNG 格式（兼容微信等应用） | RGBA 图像数据 |
| **GIF 剪贴板** | 文件路径文本 | 文件引用（可直接粘贴为文件） | 文件路径文本 |
| **视频剪贴板** | 默认不写入；`video_copy_to_clipboard: true` 时同左 | 默认不写入；开启后同左 | 默认不写入；开启后同左 |
| **全屏遮罩** | 系统全屏 | 窗口覆盖屏幕（避免独立 Space 黑屏） | 系统全屏 |
| **FFmpeg 下载** | `.7z` (~32MB) | `.zip` (ARM64/x64 自动识别) | `.tar.xz` |

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
| 视频编码 | FFmpeg（按需下载，H.264 MP4） |
| 全局快捷键 | rdev |
| 剪贴板 | arboard + osascript (macOS) |
| 7z 解压 | sevenz-rust2 (Windows FFmpeg) |
| 系统托盘 | Tauri 内置 |

## 构建

```bash
npm install
npm run tauri dev    # 开发
npm run tauri build  # 打包
```

## 配置

首次启动自动生成 `~/.sip-cc/config.json`。下面按 **GIF** 与 **视频** 分组说明；完整示例见文末。

**版本与升级**：`version` 字段等于**当前应用版本**（与安装包一致）。若本地文件里的 `version` 与当前应用不一致（含旧版无此字段），启动时会**整表重置为默认配置**并写回，便于新版本增加配置项时统一升级；业务代码无需单独判断。

### 通用（保存目录与快捷键）

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `version` | 配置架构版本，与当前应用版本一致 | 见安装包版本号 |
| `save_dir` | 保存位置：`desktop` 或 `custom` | `desktop` |
| `custom_save_dir` | 自定义保存目录路径 | `null` |
| `hotkeys` | 快捷键（也可通过托盘「快捷键设置」修改） | 见下方 JSON |

### GIF 录制（与视频相互独立）

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `gif_fps` | GIF 帧率 | `10` |
| `gif_max_duration_secs` | 单次 GIF 最长录制时间（秒，范围 1～600） | `600`（10 分钟） |

### 视频录制（FFmpeg MP4）

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `video_fps` | 视频帧率（未选质量档时兜底；档位内会覆盖） | `15` |
| `video_crf` | 画质（越小越清晰，约 18～28） | `23` |
| `video_preset` | FFmpeg x264 编码速度预设 | `medium` |
| `video_max_duration_secs` | 单次视频最长录制时间（秒，1～7200） | `1800`（30 分钟） |
| `video_copy_to_clipboard` | 编码完成后是否把 MP4 路径写入剪贴板 | `false`（大文件建议关） |

### `config.json` 完整示例

```json
{
  "version": "0.8.0",
  "save_dir": "desktop",
  "custom_save_dir": null,
  "gif_fps": 10,
  "gif_max_duration_secs": 600,
  "hotkeys": {
    "snap": "F1",
    "gif": "F3",
    "video": "F5",
    "force_quit": "Ctrl+C,Ctrl+C"
  },
  "video_fps": 15,
  "video_crf": 23,
  "video_preset": "medium",
  "video_max_duration_secs": 1800,
  "video_copy_to_clipboard": false
}
```

可通过托盘菜单「打开配置文件」直接编辑，也可在托盘菜单中切换保存目录。

## 下载

前往 [Releases](https://github.com/1207575273/sip-cc/releases) 下载对应平台安装包。

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `*.dmg` |
| macOS (Intel) | `*.dmg` |
| Windows 安装版 | `*_x64-setup.exe` / `*.msi` |
| Windows 便携版 | `*_portable.zip` |
| Linux | `*.AppImage` / `*.deb` |

## License

MIT
