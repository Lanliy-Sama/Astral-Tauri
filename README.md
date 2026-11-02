# Astral — Tauri + Svelte 5 + Vite + JS 版

Astral 是一个基于 [EasyTier](https://github.com/EasyTier/EasyTier) 的跨平台 P2P 虚拟局域网应用。
本工程是原 **Flutter 版** 前端的完整移植（Tauri 2 + Svelte 5 + Vite + JavaScript），
**GUI 界面与功能与原版 100% 一致**：

- Material 3 动态主题（种子色 → 完整色彩令牌，与 Flutter `colorSchemeSeed` 同算法同结果）
- MiSans 字体（复刻原版 pubspec 的字重注册怪癖）
- 自绘标题栏 / 窗口控制 / 系统托盘 / 关闭最小化
- 首页四卡瀑布流 + 三态连接按钮（含 15s 进度动画）
- 房间管理（增删改排序 / 分享码导入导出 / 网络配置携带）
- 用户列表三种样式 + 网络拓扑图（SVG 贝塞尔连线 + 缩放平移）
- 服务器管理（30s 状态轮询 / 拖拽排序 / 11 协议开关）
- 魔法墙（WFP 规则组/规则，进程自动监听）、NAT 测试、端口白名单
- 设置全套（软件/更新/日志/启动/监听/VPN 网段/高级网络设置）
- zh/en 双语（沿用原版翻译文件）、深浅色主题、深色模式

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Svelte 5（runes）+ Vite 6 + JavaScript |
| 桌面壳 | Tauri 2（无系统标题栏、托盘、SQL、深链接） |
| 网络核心 | 原版 `rust/` crate（EasyTier 封装，去除 flutter_rust_bridge） |
| 持久化 | SQLite（tauri-plugin-sql，表结构与原 Isar 集合一一对应） |
| 主题 | @material/material-color-utilities（与 Flutter 同源的 M3 算法） |

## 目录结构

```
├── src/                        # Svelte 5 前端
│   ├── app.css                 # Material 3 设计系统（全局令牌 + 组件样式）
│   ├── App.svelte / main.js
│   └── lib/
│       ├── components/         # home / rooms / servers / magicwall / nat / tools / settings
│       ├── services/           # ServiceManager（状态+业务）、主题引擎、连接管理器、魔法墙
│       ├── stores/             # 15 个响应式状态（对应原 core/states）
│       ├── db/                 # SQLite / localStorage 双模式持久层
│       ├── rust/api.js         # Tauri invoke 封装 + 浏览器 Mock 模拟器
│       ├── i18n/               # zh/en（沿用原翻译 JSON）
│       └── utils/              # IP/版本/NAT/分享码等工具（逐函数翻译自 Dart）
├── src-tauri/                  # Tauri 2 桌面壳
│   ├── src/{lib,commands}.rs   # 命令层 + UDP 日志桥 + 托盘
│   ├── crates/astral-core/     # 原版 rust/ 网络核心（可复用）
│   ├── tauri.conf.json         # 960×540、最小 200×300、无标题栏
│   └── mobile-reference/       # Android VPN Service 插件参考源码
├── docs/
│   ├── PORTING.md              # 移植报告（架构映射 + 保真决策）
│   ├── MOBILE.md               # Android/iOS 构建指南
│   └── screenshots/            # 各页面运行截图
└── public/                     # MiSans 字体、图标、翻译文件
```

## 开发与构建

### 前置要求
- Node.js ≥ 20
- Rust ≥ 1.77（构建桌面端时）
- 平台依赖：Linux 需 `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`；Windows/macOS 无需额外依赖

### 浏览器预览（无需 Rust，带完整 Mock）
```bash
npm install
npm run dev          # http://localhost:1420
npm run build        # 产出 dist/
npm run preview      # 预览生产构建
```
Mock 模式完整模拟连接状态机、节点数据、NAT 检测、防火墙、魔法墙，
可在浏览器中演示与截图全部 UI。

### 桌面应用（Tauri）
```bash
npm install
npm run tauri dev    # 开发模式
npm run tauri build  # 打包安装包（Windows 下产出 NSIS/MSI）
```
> Windows 首次构建需 Npcap SDK（`Packet.lib`）：设置环境变量 `NPCAP_SDK_LIB`
> 指向 SDK 的 `Lib\x64` 目录，或解压 SDK 到 `src-tauri/crates/astral-core/../third_party/npcap-sdk/Lib/x64`。
> Linux 运行需 sudo（与原版一致）。

### 移动端（Android / iOS）
见 [docs/MOBILE.md](docs/MOBILE.md)。

## 与原版的对应关系

| Flutter | 本工程 |
|---|---|
| `MaterialApp(colorSchemeSeed, themeMode)` | `lib/theme/material3.js` + CSS 变量注入 |
| `signals_flutter` | Svelte 5 runes（$state/$derived/$effect，语义一一对应） |
| `Isar` 集合 | SQLite 表（rooms/servers/all_settings/net_config/theme_settings/mw_*） |
| `flutter_rust_bridge` API | `rust/api.js` → Tauri `invoke()`（命令同名同参） |
| Rust 事件 → UDP 9999 → LogCapture | Rust 事件 → UDP 9999 → Tauri `rust-log` 事件 |
| `Navigator.push` 子页 | `router/page-router.svelte.js` 路由栈覆盖层 |
| `IndexedStack` 保活 | MainScreen 全渲染 + display:none |
| `window_manager`（960×540 无边框） | `tauri.conf.json` + `data-tauri-drag-region` |
| `tray_manager` | Tauri 托盘（显示主界面/隐藏托盘/退出） |
| `easy_localization` | `lib/i18n`（同一份 zh.json/en.json） |

详细映射与保真决策见 [docs/PORTING.md](docs/PORTING.md)。

## 许可

沿用原项目 [PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0)。
本移植版保留原版全部版权与许可要求（见 `src-tauri/crates/astral-core` 内原版源码注释）。
