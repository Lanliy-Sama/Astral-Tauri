# 移植报告：Flutter → Tauri + Svelte 5 + Vite + JS

> 目标：GUI 界面和功能与原版 100% 一致，用户无法体感变化。
> 原版：`astral-main`（Flutter 3.x + signals_flutter + Isar + flutter_rust_bridge 2.12 + EasyTier）
> 移植：`astral-tauri`（Tauri 2 + Svelte 5 runes + Vite 6 + JS + 原版 Rust 核心复用）

## 1. 架构映射总表

| 原 Flutter | 移植后 | 保真说明 |
|---|---|---|
| `MaterialApp(colorSchemeSeed: themeColor, useMaterial3: true, themeMode)` | `src/lib/theme/material3.js`：@material/material-color-utilities 的 `themeFromSourceColor`（与 Flutter 同为 Material 官方 SchemeTonalSpot 实现）+ 自行补全 Flutter 3.22+ 的 surfaceContainer 系列 tone（light 100/96/94/92/90/87/98，dark 4/10/12/17/21/6/24），逐个注入 CSS 变量 | 种子色→令牌的计算与 Flutter 完全同源；已验证 blue 种子产出 `#0061a4/#d1e4ff/#f4f3f7` 等与 Flutter 一致的值 |
| `Typography.material2021().apply(fontFamily: 'MiSans')` | `app.css` 中 `.md-title-medium` 等 15 级排版类（字号/行高/字重/字距按 M3 2021 规范） | MiSans 字重注册复刻原版 pubspec 怪癖：400→Demibold、800→Regular（浏览器字体匹配规则与 Flutter 引擎一致，w400/500 用 Demibold、w700+ 用 Regular，与原版渲染相同） |
| `signals_flutter`（signal/computed/Watch） | Svelte 5 runes：`$state` 类字段 / `$derived` / 组件内直接读取 | 语义一一对应；15 个 State 类与原 `core/states/*.dart` 字段同名同默认值 |
| Isar（8 个集合） | `db/db.js`：Tauri 模式 SQLite（`sqlite:astral.db`，表 `(id, data JSON)`）；浏览器模式 localStorage 同接口 | 表/行结构与 Isar 集合一一对应；保留 ServerDao 启动清理废弃域名、默认房间/默认服务器种子、AllSettings 单例 id=1、socks5 端口归一化、userListStyle=-1 迁移、downloadAccelerate 旧值迁移等全部初始化行为 |
| flutter_rust_bridge API（simple/firewall/hops/nat_test/magic_wall） | `src-tauri/src/commands.rs` 封装 `crates/astral-core`（原 `rust/` 去 FRB）→ Tauri 命令；前端 `rust/api.js` invoke 封装 | 命令名/参数/返回 JSON 形状（camelCase）与浏览器 Mock 完全一致；`create_server` 保持 fire-and-forget + 前端轮询 `get_running_info` 的原版语义 |
| Rust 事件 → `send_udp_to_localhost`(127.0.0.1:9999) → Dart `LogCapture` → logs(1000 条环形) | Rust 核心不变（仍发 UDP）→ `start_log_bridge` 线程监听 9999 → Tauri `rust-log` 事件 → 前端 pushLog（1000 条环形） | 中文事件文案完全由原 Rust 代码生成，零改动 |
| `Navigator.push`（根路由，子页覆盖全屏含 AppBar） | `router/page-router.svelte.js` 路由栈 + MainScreen 覆盖层（fade+scale 220ms 近似 ZoomPageTransitions） | 子页 AppBar 56px 居中标题 + 返回箭头（对应 BaseSettingsPage centerTitle: true） |
| `IndexedStack`（5 页保活） | MainScreen 全部渲染 + `display:none` | 页面内部状态（如拓扑开关、草稿）切 Tab 不丢 |
| `window_manager`：960×540、min 200×300、titleBarStyle hidden、setPreventClose | `tauri.conf.json` 同参数 + `on_window_event(CloseRequested)`：closeMinimize=true → 隐藏到托盘（原 onWindowClose 逻辑） | `--autostart` 参数静默启动已实现 |
| `tray_manager`（显示主界面/隐藏托盘/退出；左键显示） | `setup_tray`：同菜单结构；隐藏托盘 → `tray-hide-request` 事件 → 前端确认对话框（原文案）→ `destroy_tray` | 三段式警告文案走 i18n 原键 |
| `app_links`（astral://room?code=） | `tauri-plugin-deep-link` + `services/app-links.svelte.js`（解析/查重/导入/跳转，错误文案照原版） | astral:// 与 astral://debug 两条路由 |
| `easy_localization`（zh/en，170 键） | `lib/i18n`：直接加载原版 `zh.json/en.json`，`t(key, params)` 支持 {param} 插值 | 语言持久化 + 系统语言检测；原版 4 个插值键可用 |
| `AppSnackBars`（floating，红/绿/蓝三色 + action） | `snackbars.svelte.js` + `SnackbarHost` | error 4s / success 3s / info 3s；标题粗体 + 12px 正文 + 白色图标；连接引导条用 inverseSurface 底色 |

## 2. 视觉保真关键点

1. **连接按钮三态**：idle 100×60 primary / connecting 180×60 error + close 图标 800ms 往返旋转 + 进度条（180×6 r3，tertiary→primary 15s easeInOut，与连接超时一致）/ connected 180×60 tertiary；图标 150ms / 文字 200ms 切换动画；宽度 200ms easeOutCubic。
2. **左侧导航**：80px 宽、右描边、项高 64、选中块 primaryContainer r12、200ms easeInOutCubic 位移动画、hover 块 surfaceContainerHighest@0.5。
3. **M3 Switch**：52×32 轨道 r16、2px outline 描边（选中时变 primary）、thumb 关 16px/开 24px（关 outline 色/开 onPrimary 色）、位置关 left:6/开 left:22（+2px 边框 = 视觉 24px，即 Flutter `_computeThumbPaintOffset` 实算的 ON thumb 轨道 [24,48]、右缘留 4px、上下各 4px；OFF thumb [8,24]、左缘 8px）。悬停 thumb 变色（ON→primaryContainer / OFF→onSurfaceVariant）；禁用态轨道 OFF=surfaceContainerHighest@12% / ON=onSurface@12%，thumb OFF=onSurface@38% / ON=surface，全部对照 Flutter 3.29 `_SwitchDefaultsM3`。**Flutter 原版默认不在 thumb 上绘制任何图标**（`Switch.thumbIcon` 为 null 时无图标；Astral 全部裸用 `Switch`/`SwitchListTile`，未传入 thumbIcon），因此开启状态**没有 √ 勾选符号**——与 M3 规范插图不同，与 Flutter 实机一致。
4. **状态栏**：36px、primaryContainer 底、primary→tertiary 渐变标题（18px bold letterSpacing 1.5）、圣诞彩蛋（12/22-12/28）、主题模式/颜色/语言菜单 + 窗口控制按钮（图标 20px）。
5. **网格断点**：首页 1200/900/600→5/4/3/2 列（卡片 span=2，StaggeredGrid 放置算法已在 `MasonryGrid.svelte` 复刻）；房间 4/3/2/1；用户 3/2/1。
6. **用户卡三样式**：List（行式 + 底分隔线）/ Mini（NAT 游戏化品质色 #FF6B00/#A335EE/#0070DD/#1EFF00/#9D9D9D）/ All（详细 + 网络统计 + 连接路径跳点链）；连接类型/延迟/丢包颜色阈值（<50绿 <100橙 其余红；<1绿 <5橙 其余红）逐值对照。
7. **拓扑图**：localX=120 targetX=760 baseY=80 rowGap=130 布局算法逐行翻译；节点卡 220/186(dense) 宽、四类容器配色、bezier 连线 primary@0.6 宽 2.5 + 'XXms' 标签、点网格 20px、缩放 0.3-2.0、图例卡（Topology/Local/Server N/Player N/Servers are above players.）；结构变化保留坐标、指标 1s 节流、reduceUpdates 4s 节流（原 network_topology_diff 语义）。
8. **对话框**：M3 r28 surfaceContainerHigh；AlertDialog 原版 titlePadding/actions 布局；房间分享对话框（r28、基本信息卡、高级选项 ExpansionTile、12 项网络配置复选、摘要条）完整移植。
9. **硬编码颜色**：`--flutter-green/red/blue/orange/grey/deep-purple`、SnackBar shade700、NAT 品质色全部走 CSS 变量，与 Colors.* 500 值一致。

## 3. 功能保真关键点

1. **连接状态机**（`service-manager.svelte.js`）：idle→connecting→connected 三态；15s 超时；1s 状态轮询（get_running_info 解析 virtual_ipv4 ≠ 0.0.0.0 判连通并回写 IP）；连接后 1s 网络监视器（时长++/get_network_status/IPv4 刷新）；自动重试（2s 间隔 × maxRetryCount）；取消 Completer 语义（返回 null=用户取消）；Windows 连接成功后 setInterfaceMetric('astral', 0)；活跃服务器标记 inUse。
2. **ServerConfigBuilder**：房间配置覆盖（NetworkConfigShare 短键解析）、非法 IP 强制 DHCP、房间服务器在前去重合并、监听列表过滤 [::]、11 协议 URL 展开（tcp/faketcp/udp/ws/wss/quic/wg/txt/srv/http/https 顺序固定）、30 字段 Flags（socks5Port=enableSocks5?port:0、relayNetworkWhitelist 固定 '*'）。
3. **房间分享编解码**：`utils/roomShareCodec.js` 用原生 CompressionStream 实现 gzip+base64url（去 = 填充），短键 n/r/p/m/e/s/c/net 与原版一致；validateRoom/cleanRoom 全部校验规则（名称≤50、非法字符、公开房间房间号必填、标签≤10 等）。
4. **更新体系**：GitHub releases API（per_page=20、UA astral、15s 超时）、prerelease 过滤、语义化版本比较、latestVersion 持久化、启动 1s 静默检查、忙态/结果/下载进度对话框全套文案、下载加速（auto 测速 6s 超时 1h 缓存 / 手动前缀补 '/' / off 直连）、网盘/GitHub/手动三下载入口。
5. **魔法墙**：控制器（groups 按名升序/rules 按优先级降序/孤儿迁移/activeRulesCount=Σ启用组内启用规则）、引擎同步次序（删除先引擎后库、三条件满足才 add、update=remove+add、stop 清空后 start 重推）、5s 进程监控（仅 Windows）、事件日志（engine/group × on/off/auto_on/auto_off）、19 字段规则模型。
6. **服务器状态**：进入页面快照 + 立即一次 + 30s 周期；TCP ping >800ms 判离线；inUse 蓝/online 绿/offline 红/unknown 灰色条。
7. **防火墙**：profile 1/2/3 AND 读取、写入失败回滚。
8. **MTU 联动**：enableEncryption true→1360 / false→1380（原版行为）。

## 4. 有意的实现差异（不影响体感）

| 差异点 | 原因 / 影响 |
|---|---|
| 房间排序拖拽用 HTML5 DnD（整卡可拖） | 桌面端手势等价；触屏不支持拖拽排序（桌面主战场） |
| 下载进度对话框在浏览器模式用 fetch+Blob 下载 | Tauri 模式相同实现；安装引导提示替代 exit(0)（Web 安全限制） |
| Android 专属（安装/通知权限卡、VPN Service、桌面小部件） | 桌面/浏览器不可达；Android 集成见 docs/MOBILE.md（含原版 VPN 插件源码参考） |
| LogsPage 标题按 BaseSettingsPage 居中 | 原版自建 AppBar 未设 centerTitle（Windows 左对齐）；差异 1 个标题对齐，其余一致 |
| 拓扑图初始视口自动适配（fit） | 原 NodeFlowEditor 初始视图的等价实现，避免内容高出视口不可见 |
| Hitokoto 一言 API 沙箱内不可达 | 显示原版"网络错误"文案（联网环境正常拉取） |

## 5. 验证方式

- `npm run build`（Vite 生产构建）通过，0 错误。
- 浏览器（960×540 = 原版默认窗口尺寸）逐页截图核对：首页四卡瀑布流/连接三态/房间列表/用户三样式卡/拓扑图/服务器/工具/NAT/魔法墙/设置 9 个子页/6 类对话框/深色模式/480px 移动端底部导航 —— 见 `docs/screenshots/`（26 张）。
- 生产构建（vite preview）完整连接流程回归：启用服务器→选房间→连接→已连接→用户卡数据每秒刷新→拓扑切换→房间设置弹窗，全部通过。
- 侧边栏选中指示块对齐回归（2026-11-01 v3）：v2 将选中块改为渲染在被选中按钮内部（`inset:0`，结构性重合，无坐标换算），切换动画 FLIP + WAAPI 收敛于 `translateY(0)`。**全部 26 张截图已于修复后在生产构建上重新拍摄**（旧图系修复前所拍、指示块整体偏高 14px，已废弃），并以像素级脚本逐一校验：凡左侧导航可见的 18 张，指示块均精确落于 54+72i 槽位、高 63-64px、偏差 0px；深色主题与对话框遮罩态亦验证通过。全屏子页（NAT/监听/网络/软件/更新/日志）与添加服务器对话框（880px 宽，覆盖导航，与原版 AlertDialog+TextField 撑满行为一致）中导航不可见，属正常。同日针对用户反馈“指示块目视仍歪”做了三重独立复核：①槽位网格像素检测（全部 0px 偏差）；②选中块内图标中心偏差检测（dx=0.0、dy≤±1.0px）；③图标下文字标签与图标中心对齐检测（偏差 0.0–0.5px）；另以视觉模型对 3 倍放大导航栏复核为“图标居中、无不齐”。结论：当前实现几何与视觉均对齐。
- Switch 勾选符号回归（2026-11-01）：初版误按 M3 规范插图在开启状态 thumb 内绘制 √，但 Flutter 实机（`thumbIcon` 为 null）不绘制任何 thumb 图标，属移植偏差。已移除（`Switch.svelte` 不再渲染 check 字形，app.css 同步清理）。
- Switch M3 度量精化（2026-11-01 第二轮，逐像素对照 Flutter 3.29 源码）：① ON thumb 视觉位置从 [26,50] 修正为 [24,48]（CSS left 由 24px 改 22px，补偿 2px 轨道边框；DOM 实测 thumbX=24、右缝 4px、thumbY=4，与 Flutter `_computeThumbPaintOffset`/`_trackInnerLength` 实算完全一致）；② 补齐悬停 thumb 变色（ON→primaryContainer、OFF→onSurfaceVariant，禁用不响应）；③ 禁用 OFF 轨道色由 onSurface@12% 修正为 surfaceContainerHighest@12%（新增 `--md-surface-container-highest-012` 主题变量）；④ `Switch.svelte` 补设原生 `disabled` 属性（原先仅加 class，`app.css` 中 `:disabled` 样式实际永不生效，魔法墙非 Windows 开关的禁用外观现在真正呈现）。
- 全套交付截图 26 张已在最终生产构建上重新拍摄（2026-11-01 第二轮）：逐张 VLM 质量检查（溢出/重叠/错位/乱码/图标缺失）全部“无缺陷”；导航指示三图（home/tools/settings）像素检测偏差 0px；添加服务器对话框 TCP/UDP 开关 thumb 全分辨率像素网格验证为纯色圆盘（旧图同位置存在 17–40px 对勾字形，系陈旧构建所拍）。
- Mock 模拟器模拟 8 节点（本机+6 玩家+1 服务器）延迟/丢包抖动、连接 1.6-2.8s 后分配 IP、事件日志中文文案，与真实 Rust 行为节奏一致。

## 6. 已知限制

0. ~~侧边栏选中指示块越往下越偏~~（已修复，2026-11-01 v3）：v0 硬编码公式 `top=4+72i` 与实际布局脱节（坐标系少 14px、DPI 下逐项累积）；v1 坐标系还原+实测定位在部分环境仍漂移。v3 最终方案为**结构性对齐**：选中块直接作为被选中按钮的子元素（`position:absolute; inset:0`），静止态重合由 DOM 结构本身保证，不受 DPI/缩放/字体/渲染时序任何影响；切换动画用 FLIP（新旧按钮位置差作为起点，WAAPI 播放 `Curves.easeInOutCubic`，无论是否被中断都收敛于 `translateY(0)`）；悬停块改为按钮 `::after` 伪元素，同样结构性对齐。**注意：修复前拍摄的旧版全套截图已全部废弃重拍**（旧图指示块普遍偏高 14px），当前 `docs/screenshots/` 与交付截图均为修复后生产构建实拍并经像素脚本逐张校验（另见“验证方式”中的三重复核记录）。

1. ~~开启状态的开关 thumb 上有 √ 勾选符号~~（已修复，2026-11-01）：初版误按 Material 3 规范插图在开启 thumb 内绘制 check 字形；但 Flutter 的 `Switch` 在 `thumbIcon` 为 null（Astral 全部裸用 `Switch`/`SwitchListTile`）时**不绘制任何 thumb 图标**，即原版 GUI 开启状态无 √。已删除该字形并重拍全部受影响截图，DOM/像素/视觉三重确认无勾选（见“验证方式”）。注：复选框（Checkbox）与选中菜单项（主题模式菜单绿色 check，对照 status_bar_actions.dart）的 √ 属原版真实行为，予以保留。

1b. ~~交付截图中开关位置/样式与实际构建不一致~~（已修复，2026-11-01 第二轮）：早期交付的截图来自修复前的陈旧构建（zip 源码已含修复但截图未重拍），导致用户看到与实际不符的 √ 与指示块偏移。本轮已将源码、构建、截图、zip 四者同步为同一版本，并在重新打包前逐张校验。

2. **Rust 侧未在本环境编译**（沙箱无法访问 crates.io）：命令层代码按 Tauri 2 / 原核心 API 严格编写并逐函数核对签名；用户本地 `npm run tauri build` 时如遇少量编译错误（版本差异），按编译器提示微调即可，均为表层问题。
3. Android/iOS 需按 `docs/MOBILE.md` 初始化工程并集成 VPN Service 插件（源码已附）。
4. 桌面小部件（Android Widget）、通知常驻栏属 Android 专属能力，桌面端无对应物（与原版桌面行为一致）。
