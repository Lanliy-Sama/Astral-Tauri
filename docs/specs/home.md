# Home 模块规格（源码核对版）

## HomePage 整体结构
```
Scaffold
 ├ body: SingleChildScrollView
 │   └ Padding(14,14,14,14+bottomInset)
 │       └ StaggeredGrid.count(crossAxisCount, mainAxisSpacing:8, crossAxisSpacing:8)
 │           [UserIpBox, QuickNetworkConfig, AboutHome, HitokotoCard, 全宽100px空白占位]
 └ FAB: ConnectButton（endFloat 右下角 16px 边距）
```
- 列数（窗口宽）：≥1200→5；≥900→4；≥600→3；否则 2。卡片 widthSpan=2。
- HomeBox 外壳：Card r8（isBorder=false 时 r1），hover 边框 primary 1px 否则 transparent；内层 padding 12，宽撑满，高度自适应。

## UserIpBox（widthSpan=2）
Column(start)：
1. 标题行：`Icon(Icons.person, primary, 22)` + 8 + `t('user_info')`(18, w400) + Spacer + 非 idle 时"已锁定"徽章（bg secondaryContainer r12 padding h8v4，文本 t('locked') onSecondaryContainer 12 w300）
2. 14 间距
3. 用户名 TextField：label t('username')，hint t('username_hint')，prefixIcon person(primary)，OutlineInputBorder，contentPadding v12h12，floating always；enabled = idle；onChanged → sm.updatePlayerName；初始 text = playerState.playerName
4. 14 间距
5. 房间选择器（InkWell+InputDecorator）：label t('select_room')，prefixIcon apartment，suffixIcon menu(primary)；点击（仅非 connected）→ CanvasJump.show → 选中回调 room.setRoom(room)；显示 selectedRoom?.name ?? t('select_room_hint')；connected 时 disabled
6. 9 间距
7. 虚拟 IP 行（高 60 Row）：Expanded TextField（label t('virtual_network_ip')，hint t('virtual_network_ip_hint')，prefixIcon lan(primary)，errorText: !dhcp && !validIP 时 t('invalid_ipv4_error')；enabled = !dhcp && idle；onChange → updateIpv4+校验）+ 8 + Column[Switch(dhcp, 仅 idle 可改) + 文本 t('automatic')/t('manual') 12px]
8. dhcp 时：8 上边距 + t('auto_assign_ip_notice') 12px；否则 12 高空白

IPv4 校验：`isValidIPv4WithMask`（可选 /mask，4 段 0-255，mask 0-32）。

## CanvasJump 房间选择弹窗
- AlertDialog r16：标题 Row[meeting_room(primary) + 8 + '选择房间'(18, primary)]；内容 w=screenW/1.2 h=screenH/2+12：搜索框（hint '搜索房间'，prefixIcon search 24，OutlineInputBorder v12）+ 房间列表（每项 Container r16，hover bg primaryContainer@0.12 + border primary 1.5，否则 surfaceContainerHighest@0.95(亮)/0.15(暗)；ListTile contentPadding h16，title 房间名 16 onSurface，subtitle '加密房间'/'开放房间' onSurfaceVariant，trailing chevron_right primary@150alpha；点击 onSelect(room)）；actions '取消'(16, primary)。

## QuickNetworkConfig（widthSpan=2）
Column(start)：标题行 tune(primary,22)+8+'快捷网络配置'(18,w400，硬编码)；14；
1. 强制中转开关容器（padding h12v12，bg surfaceContainerHighest@0.3，r8，border outline@0.2 1px）Row：图标盒（padding 8，bg disableP2p? primaryContainer : surfaceContainerHighest，r8，Icons.route 20，色对应 on 色）+12+Expanded Column['强制中转'(15,w500,onSurface) + 2 + '禁用P2P直连，所有流量经服务器中转'(12, onSurface@0.6)] + 8 + Switch(disableP2p)→networkConfig.updateDisableP2p
2. 仅 Windows：8 间距 + 广播转发开关（同款，icon settings_input_antenna，'广播转发'，副文 'Windows：将局域网 UDP 广播转发到虚拟网（通常需管理员权限）'，Switch enableUdpBroadcastRelay）
3. 仅 Windows：8 间距 + 防火墙开关（icon shield 20，标题 t('firewall')，副文随 firewallState：t('firewall_enabled')/t('firewall_disabled')，Switch → firewall.setFirewall，失败回滚）

## AboutHome（widthSpan=2）
标题行 info_outline(primary,22)+8+t('about')(18,w400)；16；
- 软件版本行 Wrap(8)：smartphone(20,primary) + t('software_version')+": "(w700) + 版本值(secondary；有新版显示 "cur -> latest" + arrow_upward 16 primary)
- 4；内核版本行：平台图标(parsePlatformVersion, 20, primary) + t('kernel_version')+": "(w700) + 版本号(secondary) + 平台徽章（padding h6v2，bg primaryContainer r4，onPrimaryContainer 12 w500）
- 软件版本来自 sm.appVersion；内核版本 rust.easytierVersion()。

## HitokotoCard（widthSpan=2）
API GET https://v1.hitokoto.cn/ → {hitokoto, from}；失败文案 '获取失败'/'网络错误'；初始 '加载中...'；fallback '暂无内容'/'未知来源'。
标题行 format_quote(primary,22)+8+'一言'(18,w400)+Spacer+刷新 IconButton(refresh 18 primary, tooltip '刷新一言')；16；加载态 Row[16×16 CircularProgressIndicator(stroke 2) + 8 + '加载中...'(14, onSurface@0.6)]；内容态 Column[正文(14, onSurface, 行高1.4) + 6 + '—— from'(12, onSurface@0.6, 斜体)]。

## ConnectButton
```
Padding(16) Column(min,end)：
 ├ 进度区 SizedBox(h14,w180)：AnimatedSlide+Opacity(300ms, connecting 显示) Container(180×6, r3, bg surfaceContainerHighest, mb8) 内渐变条(tertiary→primary, 15s TweenAnimationBuilder)
 └ AnimatedContainer(200ms easeOutCubic, w: idle?100:180, h:60) FAB.extended(extendedPadding h2)
```
三态：idle[bg primary fg onPrimary icon power_settings_new_rounded 文本 '连接'(16,w500)]；connecting[bg error fg onError icon close_rounded 旋转(800ms repeat reverse) '点击取消' 进度条上滑显示]；connected[bg tertiary fg onTertiary icon link_rounded '已连接']。
图标切换 AnimatedSwitcher 150ms fade；文字切换 200ms。
点击：idle→connect 流程（connectTargetIssue 检查：noRoom→SnackBar t('select_room_first')+action t('go_select_room') 跳 room tab；noneEnabled→t('enable_server_first')+t('go_enable') 跳 servers；noServers→t('add_server_first')+t('go_add')；SnackBar bg inverseSurface 白字 action）；Npcap 检查（Windows+FakeTCP）→ Npcap 对话框（'需要 Npcap 驱动'/'检测到当前连接包含 FakeTCP 服务器。\nWindows 需要先安装 Npcap 驱动后才能使用 FakeTCP。\n\n是否前往 astral.fan 查看安装教程？'，'取消'/'查看教程'→openExternal https://astral.fan/quick-start/download-install/）；sm.connect(isManual:true) 返回 false → SnackBar '连接失败'/'请检查网络后重试，并确认房间与服务器配置无误'(error 4s)；connecting→cancelConnection；connected→disconnect。

## 更新 UI
- UpdateChecker：GET https://api.github.com/repos/ldoubil/astral/releases?per_page=20（headers Accept vnd.github.v3+json, UA astral, 15s 超时）；beta=false 跳过 prerelease；结果 UpdateCheckResult{kind, version, releaseNotes(body||'新版本已发布'), releasePage, releaseInfo}。
- checkAndPresent：showBusy 时先弹忙对话框（'检查更新' + LinearProgress + '正在从 GitHub 获取版本信息…' barrierDismissible:false）；结果弹 UpdateDialog：title '发现新版本: v' / 最新版本；content '更新内容:'(8) + notes(14)；actions '稍后再说' + （非最新且有网盘）'网盘下载'(openExternal astral.fan 页) + 'GitHub下载'(handleDownload) 或 '手动下载'(releasePage)；最新 '确定'。
- handleDownload：桌面找 asset astral-windows-x64-setup.exe 的 browser_download_url；找不到 error '下载失败'/'未找到当前平台的安装包'。DownloadProgressDialog：状态机 正在准备下载…→正在选择下载线路…(resolveAcceleratedUrl：auto 从 [https://gh.xmly.dev/, https://gh-proxy.org/, https://ghfast.top/] 测速 6s 超时选最快，缓存 1h)→正在下载更新…(progress)；title 正在下载更新/下载失败/下载完成；完成显示 check_circle(green,48)+'文件已下载到: name'+'是否立即安装？'；actions 取消下载/取消/立即安装/确定。浏览器模式可用 <a> 触发下载。

## 硬编码中文清单（保持原样）
快捷网络配置/强制中转/禁用P2P直连，所有流量经服务器中转/广播转发/Windows：将局域网 UDP 广播转发到虚拟网（通常需管理员权限）/一言/刷新一言/加载中.../暂无内容/未知来源/获取失败/网络错误/选择房间/搜索房间/加密房间/开放房间/取消/连接/点击取消/已连接/连接失败/请检查网络后重试，并确认房间与服务器配置无误/需要 Npcap 驱动/…查看教程/打开失败/无法打开教程页面，请手动访问 astral.fan/检查更新/正在从 GitHub 获取版本信息…/发现新版本/更新内容/稍后再说/网盘下载/GitHub下载/手动下载/确定/正在下载更新/下载失败/下载完成/正在准备下载…/下载进度/取消下载/文件已下载到/是否立即安装？/立即安装/无法打开安装文件

## 关键数值
页面 padding 14；网格间距 8；卡片 r8 p12 hover 边框 1px；断点 1200/900/600；标题 18 w400；副标题 15 w500；说明 12 @0.6；徽章 r12 h8v4；开关行 r8 p12 图标盒 r8 p8 icon20；FAB idle 宽 100/其余 180 高 60；进度条 180×6 r3；动画 200/300/150ms 旋转 800ms 进度 15s。
