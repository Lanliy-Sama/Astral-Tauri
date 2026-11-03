# 移动端构建指南（Android / iOS）

Tauri 2 原生支持 Android 与 iOS。本工程前端已做移动端适配
（≤480px 底部导航、Android 小窗口紧凑状态栏、触摸目标尺寸），
移动端入口与桌面共用同一套 Svelte 代码与 Tauri 命令。

## 1. 环境准备

### Android
- Android Studio（含 SDK Platform 34+、NDK、CMake）
- JDK 17
- Rust（rustup 添加目标：`rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`）

### iOS（需 macOS）
- Xcode 15+、Xcode Command Line Tools
- `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`

## 2. 初始化与运行

```bash
npm install

# Android
npm run tauri android init      # 生成 src-tauri/gen/android 工程
npm run tauri android dev       # 连接设备/模拟器调试
npm run tauri android build     # 产出 APK / AAB

# iOS
npm run tauri ios init
npm run tauri ios dev
npm run tauri ios build
```

## 3. Android 联网实现（已集成）

Android 上 EasyTier 不能自己建 TUN，必须由系统 `VpnService` 提供 fd。
本仓库通过本地移动插件 `vpn-service-plugin/` 完成桥接：

```
vpn-service-plugin/
├── src/mobile.rs     # Rust → Kotlin（PluginHandle::run_mobile_plugin）
├── src/stub.rs       # 非 Android 平台空实现
└── android/src/main/java/com/kevin/astral/vpn/
    ├── VpnPlugin.kt        # @Command prepare / start / stop
    └── AstralVpnService.kt # 真正建立 TUN 的 VpnService
```

`src-tauri/src/commands.rs` 对外暴露三个应用级命令，前端通过
`src/lib/rust/api.js` 的 `vpnPrepare / vpnStart / vpnStop / setTunFd` 调用。

### 连接时序

1. `vpn_prepare` → `VpnService.prepare()`；未授权时拉起系统确认框，
   用户拒绝则整次连接取消（返回 `{ granted: false }`）。
2. `create_server`，**Android 上强制 `noTun = true`**，核心不再自行建 TUN。
3. 轮询 `get_running_info` 直到拿到 `my_node_info.virtual_ipv4`。
4. `vpn_start` → `AstralVpnService` 建立 TUN，返回 fd。
5. `set_tun_fd(fd)` 把 fd 交给核心（必须在实例已启动后调用）。
6. 断开时先 `close_server`，再 `vpn_stop` 回收 TUN。

### 路由与 MTU 策略

- 主路由：虚拟网段（由虚拟 IP 推导，如 `10.126.0.0/24`）
- 固定追加：`224.0.0.0/4`（组播）、`255.255.255.255/32`（广播）
- 追加路由：设置页的「自定义 VPN 网段」（`vpnState.customVpn`）
- MTU：取网络设置中的 `networkConfigState.mtu`
- **排除自身包名 `com.kevin.astral`**：避免 EasyTier 自己的 UDP 流量被
  VPN 回环，否则会形成自连接

### 调试要点

- 首次连接必须在系统弹窗中允许 VPN；拒绝后需要重新连接触发授权。
- 连接成功后 `adb shell ip addr` 应能看到 `tun` 设备与虚拟 IP。
- `adb logcat | grep Astral` 可看到 TUN 创建与 fd 传递过程。
- 若 `set_tun_fd` 报 `tun fd sender not found`，说明核心实例尚未就绪，
  即第 3 步的虚拟 IP 检测被跳过。

## 4. 平台行为开关（前端已内置）

- `isAndroid()`（`src/lib/utils/platform.js`）：设置页显示「Android 设置」卡、
  VPN 网段入口；隐藏开机自启动项；魔法墙入口隐藏；下载走 APK 架构选择。
- Android 小窗口（宽<300 或 高<400）：紧凑状态栏（32px）+ 底部导航隐藏未选中标签。
- 移动端断点：≤480px 底部导航；首页 2 列、房间 1 列、用户 1 列。

## 5. 注意事项

- `astral-core` 的 Android 目标依赖 easytier 的 tun 特性，NDK 交叉编译耗时较长（首次约 10-20 分钟）。
- 桌面小部件（原 Android Widget）不在本次移植范围；如需恢复可参考原版
  `android/app/src/main/kotlin/com/kevin/astral/AstralWidgetProvider*.kt`。
- iOS 因系统限制无法使用 VpnService 模式，需改用 NetworkExtension（原版 iOS 亦未完整实现），
  当前 `vpn-service-plugin` 在 iOS 上为返回「不支持」的空实现。
