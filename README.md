# sip-cc

轻量跨平台截屏 + GIF 录制工具。

> **命名由来**：**sip** 致敬 **Snipaste**，**cc** = **c**ode**Y**ang + **C**hina，中国开发者 codeYang 出品。

## 为什么做这个

Snipaste 好用但不支持 GIF 录制，市面上能录 GIF 的工具要么太重（Electron 套壳 100MB+），要么不跨平台。需要一个极简的工具：截屏 + 录 GIF，仅此而已。

## 功能

- **截屏**：F1 触发，全屏遮罩框选，保存 PNG + 复制到剪贴板
- **GIF 录制**：F3 触发，框选区域，支持暂停/继续/停止，流式编码边录边写
- **系统托盘**：常驻后台，右键菜单操作
- **配置文件**：`~/.sip-cc/config.json`，可调帧率、时长、保存目录
- **WAL 日志**：所有操作先写日志，崩溃可追溯

## 技术栈

Tauri v2（Rust + WebView），前端原生 TypeScript + DOM，无框架。

| 能力 | 依赖 |
|------|------|
| 屏幕捕获 | xcap |
| GIF 编码 | gif crate（流式写入） |
| 全局快捷键 | rdev |
| 剪贴板 | arboard |
| 系统托盘 | Tauri 内置 |

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| F1 | 截屏 |
| F3 | 录制 GIF |
| Space | 暂停/继续录制 |
| Esc / Enter | 停止录制 |
| Ctrl+C 双击 | 强制退出应用 |

## 构建

```bash
npm install
npm run tauri dev    # 开发
npm run tauri build  # 打包
```

Windows 上产物 `target/release/sip-cc.exe` 可直接双击运行，无需安装。

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

也可通过托盘菜单「打开配置文件」直接编辑。
