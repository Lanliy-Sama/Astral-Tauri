# Astral Tauri 移植规范（子代理必读）

本任务将 Flutter 应用 Astral 的前端 1:1 移植到 Tauri + Svelte 5 + Vite + JS。
**唯一验收标准：GUI 界面和功能与原版 100% 一致，用户无法体感差异。**

## 1. 技术纪律（违反即返工）

- **只用 Svelte 5 runes 语法**：`$state` / `$derived` / `$effect` / `$props` / `$bindable`；事件用新语法 `onclick={...}`（**禁止** `on:click` 旧语法，混用会编译失败）。
- **禁止 TypeScript**（.svelte + .js），禁止引入新 npm 依赖。
- 每个 Svelte 组件的页面特有样式放自己的 `<style>`（scoped）；全局类与令牌见 `src/app.css`，**不要修改 app.css**。
- 颜色一律用 CSS 变量：`var(--md-primary)` 等 M3 令牌；Flutter 硬编码色用 `var(--flutter-green)` 等（见 app.css `:root`）。半透明组合优先用已有的 `--md-primary-06` 等辅助变量。
- 所有用户可见文案：原版走 i18n 的用 `t('key')`；原版硬编码中文的**保持硬编码中文原样**。
- `1px = 1px`：Dart 里的 EdgeInsets/SizedBox 数值直接映射为 CSS padding/margin/gap（logical px）。

## 2. 目录与引用路径

```
src/
├── app.css                 # 全局 M3 设计系统（勿改）
├── App.svelte / main.js
└── lib/
    ├── i18n/index.svelte.js        # t(key, params) / getLocale / setLocale
    ├── theme/material3.js          # M3 色彩引擎（勿改）
    ├── stores/enums.js             # CoState/MainTab/ThemeMode/UserSortOption/... 枚举
    ├── stores/states.svelte.js     # 15 个状态类（勿改）
    ├── services/service-manager.svelte.js  # sm 单例：全部业务服务（勿改，可读）
    ├── services/theme-engine.svelte.js     # applyThemeNow（勿改）
    ├── db/db.js                    # 表存储（勿改）
    ├── rust/api.js                 # Rust/Mock API：easytierVersion/getNetworkStatus/createServer/...（勿改）
    ├── router/page-router.svelte.js  # pushPage(Component, props) / popPage()
    └── utils/
        ├── platform.js             # isTauri/isWindows/isAndroid/... 
        ├── network.js              # isValidIpAddress/isValidCIDR/pingHost/isServerNode/isBlockedServer
        ├── display.js              # parsePlatformVersion/mapConnectionType/latencyColor/lossColor/
        │                           # natMiniInfo/natFullInfo/formatSpeed/formatDuration/compareVersions/
        │                           # randomRoomName/uuidV4/randomSixDigits
        └── roomShareCodec.js       # encryptRoom/decryptRoom/validateRoom/cleanRoom/generateShareLink/
                                    # extractShareCode/networkConfigShareSummary/parseNetworkConfigShare
```

组件导入示例：
```js
import Icon from '$lib/components/common/Icon.svelte';
import { sm } from '$lib/services/service-manager.svelte.js';
import { t } from '$lib/i18n/index.svelte.js';
```

## 3. 状态与服务（ServiceManager 单例）

`sm` 暴露（字段与原 Dart state 一一对应）：
- `sm.uiState`：selectedIndex / hoveredIndex / isDesktop / isInBackground / trayHidden / goTo(tab)
- `sm.themeState`：themeColor / themeMode
- `sm.roomState`：rooms / selectedRoom
- `sm.serverState`：servers
- `sm.networkConfigState`：全部网络配置字段（dhcp/ipv4/enableEncryption/mtu/...）
- `sm.playerState`：playerName / listenList
- `sm.displayState`：sortOption / sortOrder / displayMode / userListStyle（0/1/2 枚举值）
- `sm.startupState` / `sm.updateState` / `sm.windowState` / `sm.firewallState` / `sm.vpnState`
- `sm.connectionState`：connectionState('idle'|'connecting'|'connected') / netStatus
- `sm.appSettingsState`：appName / autoRetryOnFailure / maxRetryCount / reduceAnimationUpdates / logs
- `sm.serverStatusState`：serverStatuses(map) / activeServerIds

服务方法（与原版同名）：`sm.addRoom/sm.updateRoom/sm.deleteRoom/sm.reorderRooms/sm.setRoom`、
`sm.addServer/sm.updateServer/sm.deleteServer/sm.reorderServers/sm.setServerEnable`、
`sm.updateDhcp/sm.updateIpv4/sm.updateEnableEncryption(联动MTU)/...`、
`sm.updatePlayerName/sm.updateListenList/...`、`sm.connect({isManual})/sm.disconnect()/sm.cancelConnection()`、
`sm.setFirewall(bool)/sm.updateFirewallStatus()`、`sm.checkForUpdates()`、`sm.pushLog(text)`、
`sm.startServerStatusCheck()/sm.stopServerStatusCheck()`（30s 周期）。

在组件里直接读写 sm 的状态字段即可响应式更新（Svelte 5 代理自动追踪）。
列表更新要整体替换：`sm.roomState.rooms = [...]`。

## 4. 现成组件 API

- **Icon**：`<Icon name="person" size={22} color="var(--md-primary)" filled={false}/>`
  Flutter 图标名直接用（组件内部做 Material Symbols 映射）。
- **Dialog**：`<Dialog bind:open title="标题" width={420} onDismiss>{...内容...}{#snippet actions()}<button class="md-btn md-text-btn" onclick={...}>取消</button>{/snippet}</Dialog>`
- **TextField**：`<TextField bind:value label="房间名称" prefixIcon="person" errorText="" disabled={false} multiline={false} onChange={(v)=>...}/>`
- **Switch**：`<Switch bind:value disabled onChange={(v)=>...}/>`（M3 带勾选 thumb）
- **DropdownField**：`<DropdownField bind:value options={[{value:'tcp',label:'TCP'}]} label="协议" compact={false} onChange={...}/>`
- **Progress**：`<Progress type="linear" value={0.5}/>` / `<Progress type="circular" size={24} strokeWidth={3}/>`（value=null 为不定进度）
- **SubPage**：子页面外壳 `<SubPage title="NAT 类型检测">{...}</SubPage>`；带 actions 用 `{#snippet actions()}...{/snippet}`
- **Snackbar**（snackbars.svelte.js）：`snackError(title, message)` / `snackSuccess` / `snackInfo` / `snackErrorCopy`
- **全局 CSS 类**：`.md-card`(r12, surfaceContainerLow) `.md-list-tile`(+dense/two-line/three-line) `.md-switch-list-tile` `.md-btn`+`.md-filled-btn/.md-elevated-btn/.md-outlined-btn/.md-text-btn/.md-tonal-btn` `.md-icon-btn` `.md-fab`(+.extended/.small) `.md-divider` `.md-menu` `.md-text-field` `.md-dropdown` `.md-segmented` `.md-expansion` `.md-checkbox` `.md-radio` 排版类 `.md-title-medium` 等
- **路由**：`pushPage(NatTestPage)` 打开子页（覆盖全屏，带 56px AppBar）；`popPage()` 返回。

## 5. 视觉保真关键点（务必逐条对照原 Dart 文件）

1. **字号/字重/间距/圆角逐值对照**：例如原版 `fontSize: 18, fontWeight: FontWeight.bold` → `font-size:18px; font-weight:700`。
2. **Flutter 字重怪癖**：MiSans 注册为 400=Demibold、800=Regular。CSS 已按此注册，直接写 `font-weight:400/500/700` 即可复现原渲染效果。
3. **Material 组件默认值**已对齐：Card r12、Dialog r28、FAB 56×56 r16 primaryContainer、Switch 52×32、按钮高 40 r20、TextField 高 56 r4。
4. **布局断点**（按 window.innerWidth）：
   - 首页网格：≥1200→5列 / ≥900→4列 / ≥600→3列 / 其余 2 列（卡片 span=2）
   - 房间网格：≥1200→4列 / ≥900→3列 / ≥600→2列 / 其余 1 列
   - 用户网格：≥1200→3列 / ≥900→2列 / 其余 1 列
5. **IndexedStack 保活**已由 MainScreen 处理；页面内部状态在切换 Tab 后必须保留（用组件顶层 $state）。
6. **IndexedStack 内页面高度 100%**：页面根元素用 `height:100%; overflow-y:auto` 结构（对应 Scaffold body）。
7. FAB 定位：Scaffold floatingActionButton = 右下角 `position:absolute; right:16px; bottom:16px`（页面内容滚动但 FAB 固定）。
8. 空状态居中、加载态、错误态都要做（对照原版）。
9. hover 效果对照原版（卡片边框 primary、行背景等）。
10. 所有图标 24px 默认；导航/状态栏特殊尺寸按原版。

## 6. 数据与功能行为

- 数据全部经 `sm`（已接 SQLite/localStorage 双模式持久层）。
- Rust 能力经 `src/lib/rust/api.js`：浏览器 mock 已模拟连接状态机/节点数据/NAT 测试/防火墙/魔法墙。**不要自己另造 mock**。
- 房间分享编解码用 `roomShareCodec.js`（gzip+base64url 已实现）。
- 剪贴板：`writeClipboard/readClipboard`（rust/api.js）。
- 外部链接：`openExternal(url)`。
- 连接流程 UI（ConnectButton）直接调 `sm.connect/sm.cancelConnection/sm.disconnect`，状态读 `sm.connectionState.connectionState`。
- ping 服务器状态：`pingHost(url)`（>800ms 视为离线，已内置）。

## 7. 交付要求

- 你只创建/修改分配给你的文件（见任务说明），不要动其它模块。
- 写完后自查：导入路径正确、无旧事件语法、无 TypeScript、无未定义变量。
- 报告：列出创建的文件、对照原版做过的保真决策、任何存疑点。
