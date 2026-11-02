# Rooms 模块规格（源码核对版）

## RoomPage
```
Scaffold
├ body Column:
│  ├ connected && selectedRoom!=null 时：当前房间卡（Card margin(16,16,16,8) r8，hover border primary 1px；ListTile contentPadding h16v8：
│  │   title '当前房间: ${name}'；subtitle '连接状态: 已连接|连接中|未连接' + connected 追加 ' (点击分享房间)'；
│  │   onTap 仅 connected → 分享弹窗 copyShareLink(linkOnly:true)）
│  └ Expanded：connectionState != idle → UserPage；idle → 房间瀑布流
└ FAB（仅 idle）：Row(end)[FAB sort(tooltip 排序) 16px FAB paste 16px FAB add]（heroTag room_sort/paste/add；图标 sort/paste/add）
```
- 瀑布流：CustomScrollView+SliverPadding(16)+SliverMasonryGrid.count(列数, 间距8)；列数 ≥1200→4 ≥900→3 ≥600→2 否则1；尾部 spacer safeBottom+20。
- 空列表无专门空态。

## RoomCard
- Card：选中 bg = HSL(primary).withLightness(0.95)(亮)/0.10(暗)；未选中 surfaceContainerLow；r12；边框 (selected||hovered)?primary:transparent，宽 (hover&&!selected)?1:2；InkWell r12 onTap → room.setRoom(room)
- 内容 Padding(16) Column(start)：
  - Row(spaceBetween)：左 Expanded[选中 check_circle(primary,20)+右8 + 名字(18,bold,ellipsis)]；右侧图标组（IconButton 20px, padding 0）：share(tooltip '分享房间')+8 delete(选中时禁用 outline 色 tooltip '不能删除正在连接的房间'；否则点击确认对话框 '确认删除'/'确定要删除这个房间吗？'/'取消'/'删除'(red) → deleteRoom；tooltip '删除房间')+8 edit(tooltip '编辑房间' → EditRoomDialog)+8 加密图标 lock(red)/lock_open(green) 20
  - 4
  - Row(spaceBetween)：'类型: 保护|不保护'(onSurfaceVariant)；有 servers 时徽章 Container(h8v2, bg primary@0.2, r4)'携带服务器'(primary, 12)

## RoomReorderSheet（FAB sort）
- >600：Dialog 400×600；否则 DraggableScrollableSheet(0.7/0.5/0.9, bg surface, 顶圆角28)
- 标题区 Padding(24,14,8,16)：sort(primary,24)+8+'房间排序'(18,primary,w500)；4+'拖拽房间卡片来调整显示顺序'(bodyMedium onSurfaceVariant)
- 列表：ReorderableListView；项 Padding(v4) 容器 r16 bg surfaceContainerHighest，hover border primary 1.5；ListTile contentPadding h12，title 名(16,onSurface)，subtitle '加密房间'/'开放房间'；拖拽手柄=整卡 ReorderableDragStartListener
- 底部按钮 Padding(24) Row[Expanded OutlinedButton '取消', 16, Expanded FilledButton '确认']（padding v16, r12）

## RoomSettingsSheet（UserPage FAB bar_chart）
- >600：Dialog maxW400 maxH600；sheet 0.5/0.4/0.9 r28
- 标题 bar_chart_outlined(primary,20)+8+'房间设置'(16,primary,w500)；4+'点击下方设置项进行配置，所有更改将实时生效'(bodySmall onSurfaceVariant)
- ListView(h16)：4 区（区标题 p(8,16,8,8) 14 bold primary；Wrap(spacing8,runSpacing8) 选项按钮 IntrinsicWidth h32 TextButton padding h30，选中 bg primary fg onPrimary border primary，未选 bg surfaceContainerHighest fg onSurfaceVariant border outline，r5 文本13）：
  1. 显示模式：简约/详细/列表 → setUserListStyle(0/1/2)
  2. 用户显示：默认/用户/服务器 → setDisplayMode
  3. 用户排序：默认/延迟/用户名 → setSortOption
  4. 排序方式：升序/降序 → setSortOrder

## UserPage（连接时显示）
```
Scaffold
├ FAB Column(end)：FAB topology_toggle（icon _showTopology?list:hub，tooltip '列表视图'/'拓扑图'，切换局部 state）16px FAB room_settings(bar_chart, tooltip '房间设置' → RoomSettingsSheet)
└ body：
   - != connected：居中 cloud_off_rounded(48,primary)+16+'无数据'(bold,primary)
   - netStatus==null||nodes空：people_outline(64,primary@0.6)+16+'房间内暂无成员'(18,bold,primary)+8+'当前没有其他玩家连接到房间'(onSurface@0.7)
   - _showTopology → NetworkTopologyView
   - 否则排序/过滤后按 userListStyle 渲染
```
- 排序：latency 按 latencyMs 数值 / nameLength 按 hostname 长度；ascending 正序 descending 反序；none 不排。
- 过滤 displayMode：users→排除 isServerNode；servers→仅保留；all→全部。显示名去掉 'PublicServer_' 前缀。
- list 样式：SliverPadding(8,0,8,8)+SliverList，每项 ListUserCard(showDivider=非最后)
- simple/detailed：Padding(12,0,12,0) 网格 mainAxisSpacing8 crossAxisSpacing8，列数 ≥1200→3 ≥900→2 否则1；每项 MiniUserCard / AllUserCard

## ListUserCard
- hover bg primary@0.06；padding h12v10；底边框 outlineVariant@0.5（非最后）
- Row：person(primary,18)+8+名字(w600,14,ellipsis)+8+类型徽章(h6v2,bg 类型色,r6,白字11)+（非'本机'：8+latency ms(latencyColor,bold,12)+6+丢包%(lossColor,bold,12)）+（有IP：8+Flexible IP(secondary,12,右对齐ellipsis)）
- 点击复制 IP → snack '已复制'/'IP地址: ${ip}'(success 2s)

## MiniUserCard
- Card r8 hover border primary 1；padding h12v8；Column(min)：
  - 行1：person(primary,18)+6+名字(bold,15,ellipsis)+10+类型徽章(h6v2 r8 白字12)+[非本机：10+timer_outlined(16,latencyColor)+'XXms'(bold,13)+10+error_outline(16,lossColor)+'X.X%'(bold,13)]
  - 4
  - 行2：有IP lan_outlined(16,primary)+4+IP(secondary,13,ellipsis)+10+平台图标(16,primary)+版本(secondary,13)+[非本机且nat非空：10+NAT图标(16,nat色)+NAT文案(nat色,13,bold)]+[tunnelProto:10+router(16,primary)+格式化协议(secondary,13)]
- 点击复制 IP

## AllUserCard
- Card r8 hover border primary 1；InkWell splash primary@0.3 highlight@0.1；Container(p12,w∞)：
  - 头部 Row：Expanded[person(primary,22)+8+名字(18,bold,ellipsis)] +16+ Wrap(spacing12,runSpacing4,end)：类型徽章(h8v4,r12,icon14白+4+文本白12bold)+[非本机：Tooltip'延迟' timer_outlined(18,latencyColor)+4+'XX ms'(bold,13)；Tooltip'丢包率' error_outline(18,lossColor)+4+'X.X%'(bold,13)]
  - 8+Divider+12
  - 网络数据：无 connections 居中'无连接数据'(onSurface@0.6)；否则 wifi(20,primary)+8+'网络数据:'(bold,onSurface)+12+SizedBox(h90) PlayerCardStats(connections[0])
  - 8+Divider+8
  - 详情行（每行 icon20 primary +12+label: bold +8+值(secondary,w500,ellipsis)，间隔8）：有IP lan_outlined 'IP地址'+复制按钮 copy(18,tooltip'复制IP地址')；'ET版本' 平台图标+版本号；'NAT类型' 图标+技术文案(值色)；tunnelProto '隧道类型'；hops 非空 route(20,primary)+12+Column['连接路径:'(bold)+4+每跳 Container(h6v2,bg primaryContainer,r4)'${nodeName} (XXms, X.X%)'(12,onPrimaryContainer) 跳间4]
- PlayerCardStats：两列 Row(中缝16)：左列 primary 色 upload_rounded '累计上传' formatSpeed(tx)；10；arrow_upward_rounded '累计发送包' txPackets；右列 secondary 色 download_rounded '累计下载' rx；arrow_downward_rounded '累计接收包'。每项 icon18+6+Column[label(12,onSurface@0.7)/value(bold,14,列色)]
- formatSpeed：B/KB/MB/GB/TB 除1024；整数不带小数，否则 toFixed(2) 去尾 0

## 连接类型/颜色（peer_connection_style）
- mapConnectionType(cost, ip, thisip)：ip=='0.0.0.0'→'服务器'；ip==本机IP→'本机'；cost==1→'直链'；cost>=2→'中转'；否则'未知'
- 颜色：服务器 deepPurple；直链 green；中转 orange；本机 primary；未知 grey。图标：dns/link/swap_horiz/computer/device_unknown
- 延迟色 <50 green <100 orange 否则 red；丢包 <1 green <5 orange 否则 red
- formatTunnelProto：tcp→tcp4,udp→udp4 逗号连接

## NAT 显示
- Mini 卡（游戏化）：OpenInternet/NoPat=传奇 #FF6B00 workspace_premium；FullCone=史诗 #A335EE military_tech；Restricted/PortRestricted=优质 #0070DD verified；Symmetric=困难 #9D9D9D block；SymUdpFirewall/SymmetricEasyInc/Dec=普通 #1EFF00 circle；Unknown=未知 grey help_outline（用 utils natMiniInfo）
- All 卡（技术）：Unknown未知/OpenInternet开放网络/NoPat无PAT/FullCone全锥形/Restricted受限锥形/PortRestricted端口受限锥形/Symmetric对称型/SymUdpFirewall对称UDP防火墙/SymmetricEasyInc对称递增型/SymmetricEasyDec对称递减型；色：开放/全锥/无PAT green；受限/端口受限 orange；对称/防火墙 red；未知 grey（natFullInfo）

## 网络拓扑视图
- 布局常量：localX=120 targetX=760 baseY=80 rowGap=130；服务器行在上玩家行在下（各自按 peerId 升序），两组都有中间隔一行
- 本地节点 x=120, y=80+(totalRows-1)*130/2（totalRows<=1 时 (120,250)）；非本地 (760, 80+rowIndex*130)
- hops 中继链：本地→每跳→目标；跳节点 x=120+(640)/(hopCount+1)*(i+1)，y 与目标同行；displayName=hop.nodeName||'relay_${targetIp}'
- 节点：宽 dense(≥12节点)?186:220；头部 Container(h10v8,上圆角8) bg：local primaryContainer/server tertiaryContainer/player secondaryContainer/relay surfaceContainerHighest，fg 对应 on 色；图标 computer/cloud/person/router(18)+6+名(bold,13,ellipsis)；身体 Container(h10v8,下圆角8) bg surface border outline 1：IP行 language13+IP(11)+4+平台行 devices13+platform(11)+[latency>0:4+speed13+'Xms'(11,延迟色,bold)]；dense 顺序：延迟先 IP 后
- 连线：bezier，primary@0.6 线宽2.5；label '${latencyMs.round()}ms'；动画粒子（!reduceAnimationUpdates && !isInBackground 时 4s，否则无动画 1s）
- 画布：bg surface，点网格 size20 色 outlineVariant；缩放 0.3-2.0；pan 拖拽
- 图例（top12 left12）：Card(elev0, bg surface@0.92) p(h10v8)：‘Topology’(w700)+6+Wrap(spacing10,runSpacing6)：'Local'(computer,primaryContainer色块)+3px色块内12px图标(黑白按亮度)+4+label(11)；'Server N'(cloud,tertiaryContainer)；'Player N'(person,secondaryContainer)；两组都有时+6+'Servers are above players.'(11,onSurfaceVariant)
- 节流：结构变化才重建（保留坐标）；指标 1s 节流更新；reduceUpdates 时 4s；后台不同步
- JS 实现建议：absolute 定位卡片 + SVG bezier 连线；wheel 缩放 + 拖拽平移

## 对话框
### AddRoomDialog
AlertDialog '添加房间'：TextField '房间名称'(初始 randomRoomName()) 8 SwitchListTile '是否保护'(初始 true) 非加密时 TextField '房间号'+'房间密码'；actions '取消'/'确定'（加密: roomName/password/messageKey=uuid；addRoom）
### EditRoomDialog
customParam 非空 → '无法编辑房间'（lock48 primary + 16 + '此房间含有自定义参数'(bold) + 8 + '含有自定义参数的房间不可编辑，只能删除。如需修改，请删除此房间并重新创建或导入。' 居中 + '我知道了'）
正常：'编辑房间'：TextField '房间名称' 8 ListTile '房间类型'/'加密房间|普通房间' 非加密 TextField '房间号'/'房间密码'；'取消'/'确定'→updateRoom
### 导入（FAB paste → 粘贴弹窗）
AlertDialog '导入房间'：TextField hint '请输入分享码或链接' maxLines3 + 16 + 全宽 OutlinedButton.icon(paste,'从剪贴板导入')；actions '取消'/'导入'(shareCode 非空)
importRoom 流程错误 SnackBar(4s)：astral:// 前缀但 host!=room → '链接格式错误'/'不是有效的房间分享链接'；空 → '分享码为空'/'请提供有效的房间分享码'；<10 → '分享码格式错误'/'分享码过短或不完整，请检查是否复制完整'；解密失败 → '分享码解析失败'/'无法解析房间信息，可能是分享码已过期或损坏'；校验失败 → '房间数据无效'/错误消息；带 networkConfigJson → 确认对话框（'检测到网络配置'：settings_suggest primary + '该房间包含以下网络配置：'+摘要容器(p12,bg surfaceContainerHighest@0.5,r8,border outline@0.3,每行 bodySmall)+CheckboxListTile '应用网络配置'/'将上述配置应用到当前设备'(默认true)+8+'提示：如果不应用，仅导入房间信息'(bodySmall,outline色)；'取消'/'确定'）；成功 → addRoom + goToRoom + success '导入成功'/'已成功添加并选中房间"<name>" (已内置 N 个服务器)'
剪贴板空 → '剪贴板为空'/'请先复制房间分享码或链接'；异常 → '导入失败'
### 分享导出（room_card share / 当前房间卡点击）
- Dialog r28 bg surface：Header(p 24,24,24,16)：分享图标容器(p8,bg primaryContainer,r12,share24 onPrimaryContainer)+12+Column['分享房间'(headlineSmall w600)+房间名(bodySmall,onSurfaceVariant,ellipsis)]
- Divider；内容 p24：基本信息卡(p16,bg surfaceContainerHighest@0.3,r16,border outlineVariant)：info_outline(18,primary)+8+'房间基本信息'(labelMedium w600 primary)+12+tag(16,onSurfaceVariant)+8+Column['房间名称'(labelSmall onSurfaceVariant)+4+名(bodyMedium w500 ellipsis)]
- 24 + 高级选项 ExpansionTile（默认收起；title tune_outlined(20,secondary)+8+'高级选项'(labelLarge w600 secondary)；shape r12 outlineVariant；bg surface/collapsed surfaceContainerHighest@0.2）内：'🔗 服务器列表'+'⚙️ 网络配置'（12 项 CheckboxListTile dense：DHCP/默认协议/加密/延迟优先/禁用P2P/禁用UDP打洞/禁用TCP打洞/禁用对称打洞/数据压缩/KCP代理/绑定设备/禁用TUN）；服务器 URL 列表 CheckboxListTile(mono bodySmall ellipsis)
- 有内容时 16 + 摘要条(p12,bg secondaryContainer@0.3,r12,border outline)：check_circle_outline(18,secondary)+8+'将分享 N 个服务器 + M 项配置'(bodySmall,secondary,2行)
- 底部 actions p16：'取消' + 8 + FilledButton.icon(copy_outlined,'复制链接')→copyRoomShareLink(linkOnly)
- buildRoomToShare：勾选服务器→Room.servers；hasServers→customParam=当前毫秒时间戳字符串（此房间不可编辑）；勾选配置→networkConfigJson=短键JSON
- copyShareLink：generateShareLink(encryptRoom) 写剪贴板；success '复制成功'/'房间链接已复制到剪贴板'；异常 error '复制失败'

## 房间分享编码
- 链接：astral://room?code=<shareCode>；shareCode=base64Url(gzip(json)) 去 = 填充；短键 n/r/p/m/e/s/c/net（utils/roomShareCodec.js 已实现）
- validateRoom/cleanRoom 规则见 codec

## 数据流
- netStatus 每秒由连接监视器刷新（sm.connectionState.netStatus）；IPv4 = sm.networkConfigState.ipv4（本机判定用）
- 房间 CRUD 走 sm；分享 codec 走 utils/roomShareCodec.js

## 注意
- 无右键/长按菜单；卡点击=复制 IP
- _showTopology 是 UserPage 局部 state（Tab 切换保留）
- AllUserCard 统计永远只显示 connections[0]
- customParam 非空房间编辑被拦截
