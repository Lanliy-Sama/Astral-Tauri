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

## 3. Android VPN Service 集成（联网核心必需）

Android 上 EasyTier 需要通过 VpnService 建立 TUN。原 Flutter 版的插件源码已附：
`src-tauri/mobile-reference/vpn-service-plugin/`（`TauriVpnService.kt` + `VpnServicePlugin.kt`）。

集成步骤（对照原版 `vpn_manager.dart` 的行为）：

1. **注册 Service**：把 `TauriVpnService.kt` 拷入 `src-tauri/gen/android/app/src/main/java/com/kevin/astral/`，
   并在 `AndroidManifest.xml` 添加：
   ```xml
   <uses-permission android:name="android.permission.INTERNET"/>
   <uses-permission android:name="android.permission.FOREGROUND_SERVICE"/>
   <uses-permission android:name="android.permission.POST_NOTIFICATIONS"/>
   <service
     android:name=".TauriVpnService"
     android:permission="android.permission.BIND_VPN_SERVICE"
     android:exported="true">
     <intent-filter>
       <action android:name="android.net.VpnService"/>
     </intent-filter>
   </service>
   ```
2. **权限申请**：连接前 `vpn.prepare()`（对应原版 prepareVpn）；返回的 TUN fd 通过
   `setTunFd` 命令传给 astral-core（命令已在 `src-tauri/src/commands.rs` 注册）。
3. **路由**：`ipv4/24 + customVpn 网段 + proxyCidrs`，排除自身包名 `com.kevin.astral`
   （`disallowedApplications`），MTU 取 `networkConfigState.mtu`。
4. **通知**：连接状态常驻通知（channel `astral_connection`，id 1001）——对应原版
   NotificationService；文案与刷新节奏见 `docs/specs/settings.md` 通知一节。
5. **深链接**：`tauri.conf.json` 已配置 `deep-link.mobile: [{scheme: 'astral'}]`；
   安卓需在 gen 工程的 intent-filter 中确认 scheme 注册。

## 4. 平台行为开关（前端已内置）

- `isAndroid()`（`src/lib/utils/platform.js`）：设置页显示「Android 设置」卡、
  VPN 网段入口；隐藏开机自启动项；魔法墙入口隐藏；下载走 APK 架构选择。
- Android 小窗口（宽<300 或 高<400）：紧凑状态栏（32px）+ 底部导航隐藏未选中标签。
- 移动端断点：≤480px 底部导航；首页 2 列、房间 1 列、用户 1 列。

## 5. 注意事项

- `astral-core` 的 Android 目标依赖 easytier 的 tun 特性，NDK 交叉编译耗时较长（首次约 10-20 分钟）。
- 桌面小部件（原 Android Widget）不在本次移植范围；如需恢复可参考原版
  `android/app/src/main/kotlin/com/kevin/astral/AstralWidgetProvider*.kt`。
- iOS 因系统限制无法使用 VpnService 模式，需改用 NetworkExtension（原版 iOS 亦未完整实现）。
