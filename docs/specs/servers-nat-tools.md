# Servers / NAT / Tools 模块规格（源码核对版）

## ServersPage（BaseStatefulSettingsPage：无 AppBar，FAB add）
- initState postFrame：servers 快照 → sm.serverStatusState.startPeriodicCheck(servers, 30s)；dispose stop
- Watch(servers + serverStatuses)
- ReorderableListView.builder(padding 12, buildDefaultDragHandles:false, 整卡拖拽 ReorderableDragStartListener)：onReorder → removeAt/insert（无 -1 调整）→ sm.reorderServers
- 空态：dns_outline(64, outline) + 16 + '暂无服务器'(titleMedium outline) + 16 + ElevatedButton.icon(add,'添加服务器')
- 服务器卡：Card(margin bottom 8) ListTile：leading Container(4×40, r2, bg 状态色)；title 名(w600)；subtitle url（isBlockedServer → '***'，ellipsis 1行）；trailing Row[Switch(scale 0.8, value enable → setServerEnable) + 4 + PopupMenuButton('edit'/'delete')]
  - 'edit' 项：edit_outlined(18, 被屏蔽?outline:primary) + 12 + '编辑'（被屏蔽时 outline 色）；选中：被屏蔽 → snackInfo '不可编辑'/'此服务器不可编辑'(2s)；否则 EditServerDialog
  - 'delete' 项：delete_outline(18, error) + 12 + '删除'(error)；选中 → 确认对话框 '删除服务器'/'确定要删除服务器 "${name}" 吗？此操作不可撤销。'，'取消'/'删除'(red foreground) → deleteServer
- 状态色：online green / offline red / inUse blue / unknown outline
- ServerDialog：'添加服务器'/'编辑服务器'（confirmText '添加'/'保存'）；Form+SingleChildScrollView+Column(min)：TextField '服务器名称'(hint '输入服务器名称'，非空校验 '请输入服务器名称') 16 TextField '服务器地址'(hint '输入服务器地址'，非空校验 '请输入服务器地址'；编辑且被屏蔽 disabled+helper '此服务器地址不可修改') 24 '支持的协议:'(bold) 8 Wrap(spacing16,runSpacing8) 11 项 Row[Switch+label]：TCP/FAKETCP/UDP/WS/WSS/QUIC/WG/TXT/SRV/HTTP/HTTPS；新增默认 tcp=true udp=true 其余 false；enable 新增 false 编辑保留；actions '取消' + ElevatedButton(confirmText, bg primary fg onPrimary)
- BlockedServers：['js.629957.xyz:11012','nmg.629957.xyz:11010','nmg.629957.xyz:11010']（重复保留）精确匹配；被屏蔽 → 显示 ***、不可编辑

## ToolsPage（CustomScrollView+SliverPadding(16)）
- 节标题 '联机工具'：Padding(4,0,0,8) titleMedium w600 onSurface；12
- 工具卡 Card(margin 0) ListTile(contentPadding h20v12, leading icon24 primary, title bodyLarge w500, subtitle bodySmall onSurfaceVariant, trailing chevron_right onSurfaceVariant)；卡片间 8；末尾 32
  1. 魔法墙/高级防火墙管理/security——仅 Windows 显示 → push MagicWallPage
  2. 端口白名单/配置TCP/UDP端口访问白名单/security_outlined → push PortWhitelistPage
  3. NAT 类型测试/检测您的网络 NAT 类型/network_check → push NatTestPage

## NatTestPage（AppBar 'NAT 类型检测' centerTitle）
- initState postFrame 自动测试；state: _testResult{natTypeV4,natTypeV6,ipv4Latency,ipv6Latency}, _isTestingNat, _stunServer('stun.hot-chilli.net')
- 候选 STUN：['stun.hot-chilli.net','stun.miwifi.com','stun.l.google.com']
- _testNetwork：rust.testNetworkConnectivity(stunServer)；失败 snackError 'NAT 检测失败'/err + action '重试'(白字)
- 布局 SingleChildScrollView(p16) Column(stretch)：
  1. 三态卡：测试中 Card(p20) Row[CircularProgressIndicator(24,stroke3) 16 Column['正在检测中...'(titleSmall)/'使用多 STUN 标准探测'(bodySmall onSurfaceVariant)]]；有结果 Card(p16) Column[IPv4 compact 结果 16 Divider 16 IPv6]；失败 Card(p20) Row[error_outline(error) 16 '检测失败'(titleSmall)]
  2. 16
  3. Row：Expanded DropdownButtonFormField(label 'STUN 服务器', OutlineInputBorder r8, contentPadding h12v8, testing 时禁用) + 12 + FilledButton.icon(refresh,'检测')（testing 禁用）
- _buildCompactResult(protocol,natType,latency)：行1 Row[协议徽章 Container(h8v4,bg primaryContainer,r4)'IPv4'/'IPv6'(labelMedium bold onPrimaryContainer) + Spacer + speed(16,primary) + 4 + latency>=0?'${latency}ms':'-'(bodySmall bold)] 8 行2 Row[icon(20,色) 8 natType(bodyMedium bold 色)] 4 描述(p28 bodySmall onSurfaceVariant)
- NAT 分类（contains 模糊匹配中英文）：颜色：完全锥形/Full Cone/开放互联网/Open Internet→green；受限锥形/Restricted Cone/端口受限/Port Restricted→blue；对称型/Symmetric→orange；其他→red。图标：绿→check_circle；蓝→info；橙→warning；红→error。描述：完全锥形→'最佳连接。任何主机都可发送数据。'；受限锥形→'良好连接。只有联系过的主机可发送。'；端口受限→'较好连接。常见类型，部分联机可用。'；对称型且非防火墙→'可能影响P2P连接，需要中继服务器。'；防火墙→'UDP流量受限。请检查防火墙。'；被阻止→'UDP被阻止。请检查路由器设置。'；开放互联网→'理想状态！无NAT，直连互联网。'；其他→'无法确定。尝试更换STUN服务器。'

## 连接流程要点（ConnectButton 调用）
- connect 返回三值：true/false/null(取消)；15s 超时 disconnect；重试 2s 间隔 × maxRetryCount
- sm.connectionState.connectionState 驱动 UI；netStatus 每秒刷新

## 服务器状态
- 30s 周期（+立即一次）；ping 判定 >800ms 离线；在 activeServerIds → inUse
- ServerStatusStyle：online green offline red inUse blue unknown outline
