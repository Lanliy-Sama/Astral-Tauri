# Settings 模块规格（源码核对版）

## SettingsMainPage（ListView padding 16）
- 分组头：'网络设置'=t('network_settings')，'通用设置'=硬编码；样式 Padding(l16,b8) titleMedium primary w600；组头后 8
- 设置卡列表（Card margin bottom8 ListTile leading icon primary trailing chevron_right 点击 push 子页）：
  网络组：list_alt t('listen_list')/'管理网络监听地址'→ListenListPage；vpn_lock t('custom_vpn_segment')/'配置VPN网段'→VpnSegmentPage（仅 Android）；network_wifi '高级网络设置'/'协议、加密等高级选项'→NetworkSettingsPage
  24 间距后通用组：launch t('startup_related')/'开机启动和自动连接'→StartupPage（非 Android）；info t('software_settings')/'权限和界面设置'→SoftwareSettingsPage；system_update t('update_settings')/'自动更新和下载设置'→UpdateSettingsPage；article_outlined '日志'/'查看应用运行日志'→LogsPage
- 子页全部 pushPage()（SubPage AppBar 居中标题+返回）

## 各子页

### SoftwareSettingsPage（title t('software_settings')，Android 检查权限——桌面忽略）
Card1 头 ListTile settings '软件设置'/'软件行为描述'，divider：
- 非 Android：SwitchListTile t('minimize')/t('minimize_desc') value closeMinimize → updateCloseMinimize
- ListTile '玩家列表样式't('player_list_card')/t('player_list_card_desc') + SegmentedButton(16,0,16,12 padding)：简约/详细/列表 → setUserListStyle
- SwitchListTile '减少动画更新'/'降低拓扑图与连线动画刷新频率，减少 GPU 占用' → updateReduceAnimationUpdates
16
Card2 头 sync '连接设置'/'配置连接重试行为'：
- SwitchListTile '连接失败自动重试'/'连接失败时自动重新尝试连接' → updateAutoRetryOnFailure
- 开启时 ListTile '最大重试次数'/动态 '当前设置为 {n} 次' + Dropdown[1,2,3,5,10]（宽100 isExpanded label '{} 次'）→ updateMaxRetryCount

### UpdateSettingsPage（title t('update_settings')；AppBar action refresh tooltip t('check_update')）
Card1 头 system_update t('update_settings')/t('update_behavior_desc')：
- SwitchListTile t('join_beta')/t('join_beta_desc') → updateBeta
- beta 关时 SwitchListTile t('auto_update')/t('auto_update_desc') → updateAutoCheckUpdate
- divider + ListTile bolt t('download_acceleration')/动态副标题（关闭→t('download_acceleration_disabled')；auto 有测速→t('download_acceleration_auto_current',{mirror})；auto 未测→t('download_acceleration_auto_pending')；手动→前缀本身）chevron → 加速编辑对话框
  对话框 AlertDialog t('download_acceleration_title')：说明 t('download_acceleration_info_desc') 12 + RadioListTile×3（auto t('download_acceleration_auto') 副标题=镜像列表换行 12px；manual t('download_acceleration_manual')+TextField hint t('download_acceleration_manual_hint')；off t('download_acceleration_disabled')）+ auto 选中时 8+OutlinedButton.icon(测速中16×16 CircularProgressIndicator w2+t('download_acceleration_probing')/否则 speed icon+t('download_acceleration_reprobe'))+测速结果(13)；actions t('cancel')/t('save')（auto→'auto'；manual→补/尾缀；off→''）
16
Card2 头 update t('update_operations')/t('update_operations_desc')：
- ListTile refresh t('check_update')/t('check_update_available') → checkForUpdates（忙对话框+结果对话框，同 Home 规格）
- ListTile info_outline t('version_info')/t('version_info_desc') → 版本信息对话框（'当前版本： {v}' 8 '更新通道： Beta/Stable'，t('close')）
- ListTile history t('history_versions')/t('history_versions_desc') → HistoryVersionsPage
- ListTile cloud_download '重新下载'/'如果出现问题可以尝试重新下载！' → 强制下载检查
16
Card3 头 help_outline t('update_description')/t('update_description_desc')（静态）：
- ListTile science t('beta_version')/t('beta_version_desc')；ListTile auto_awesome t('auto_update_title')/t('auto_update_info_desc')

### LogsPage（自建 AppBar：返回 + '日志' + actions copy_all tooltip '复制所有日志' + clear_all tooltip '清空日志'）
- 复制：空 → snackInfo '提示'/'暂无日志可复制'；否则 join('\n') → '复制成功'/'已复制 {n} 条日志到剪贴板'
- 清空：确认 '确认清空'/'确定要清空所有日志吗？' '取消'/'确定' → logs=[] + snackSuccess '已清空'/'日志已清空'
- 空：居中 article_outlined(64, grey) 16 '暂无日志'(18, grey)
- 列表：ListView padding8 每条 Card(margin v2) ListTile dense leading Icon(20)（含 error/错误→error_outline red；warning/警告→warning_amber_outlined orange；否则 info_outline blue）title 12px monospace（错误红/警告橙）trailing 复制按钮 copy(16, tooltip '复制此条日志')→snack '复制成功'/'日志已复制到剪贴板'(1s)
- FAB keyboard_arrow_down tooltip '滚动到底部'（animateTo 300ms）

### HistoryVersionsPage（title t('history_versions')；action refresh tooltip '刷新'）
- GET https://astral.fan/downloads.json → [{version,url,date}]
- 加载中居中 CircularProgressIndicator；失败：error_outline(64,red)+16+'加载失败: HTTP {code}'或'加载失败: {e}'(16)+16+ElevatedButton.icon(refresh,'重试')；空：'暂无历史版本'(16)
- 列表 ListView p16 每项 Card(mb12) ListTile：leading CircleAvatar(序号 index+1 bold)；title version(16,w600)；subtitle date；trailing download；点击 openExternal(url) 失败 snackError '无法打开链接'/url

### StartupPage（title t('startup_related')）
Card1（无头）三开关：
- t('startup_on_boot')/t('startup_on_boot_desc') → updateStartup（Windows 计划任务，sm 内已调 Tauri 命令）
- t('startup_minimize')/t('startup_minimize_desc') → updateStartupMinimize
- t('startup_auto_connect')/t('startup_auto_connect_desc') → updateStartupAutoConnect
16
Card2 头 t('startup_description')（静态）：
- power_settings_new t('startup_on_boot_title')/t('startup_on_boot_info') divider
- minimize t('startup_minimize_title')/t('startup_minimize_info') divider
- play_arrow t('startup_auto_connect_title')/t('startup_auto_connect_info')

### NetworkSettingsPage（title t('network_settings')）
CardA '基础'：
- ListTile 'Astral连接协议't('p2p_hole_punching')/t('preferred_protocol') trailing Dropdown(tcp TCP/udp UDP/faketcp FakeTCP/ws WebSocket/wss WSS/quic QUIC)（包裹容器 padding h8 r8，隐藏 underline，arrow_drop_down；label 14；空值显示 'tcp'）→ updateDefaultProtocol
- SwitchListTile t('enable_encryption')/副标题复用 t('auto_set_mtu')（原版怪异映射，保留）→ updateEnableEncryption（联动 MTU）
- SwitchListTile t('latency_first')/t('latency_first_desc') → updateLatencyFirst
- SwitchListTile t('disable_p2p')/t('disable_p2p_desc') → updateDisableP2p
16
CardB 头 settings_ethernet t('advanced_network_settings')/t('advanced_network_settings_desc')：
- SwitchListTile t('disable_udp_hole_punching')/desc → updateDisableUdpHolePunching
- t('disable_tcp_hole_punching')/desc；t('disable_sym_hole_punching')/desc
- ListTile t('compression_algorithm')/t('compression_algorithm_desc') trailing Dropdown[1→t('no_compression')/2→t('high_performance_compression')] → updateDataCompressAlgo
- SwitchListTile t('enable_kcp_proxy')/desc → updateEnableKcpProxy
- t('bind_device')/desc → updateBindDevice
- t('tun_device')/t('tun_device_desc') value noTun → updateNoTun
- t('enable_socks5')/t('enable_socks5_desc') → updateEnableSocks5
- 开启时：ListTile t('socks5_port')/t('socks5_port_desc') trailing 100宽 TextFormField(数字右对齐 r4 isDense key=ValueKey(端口)) 提交校验 0<p<=65535 → updateSocks5Port；ListTile t('socks5_address_hint',{port})（primary w500）
16
CardC '跃点'：
- SwitchListTile t('auto_set_hop')/t('auto_set_hop_desc') value autoSetMTU（存 AllSettings）→ sm.setAutoSetMTU
- ListTile list t('view_hop_list') chevron → rust.getAllInterfacesMetrics() → AlertDialog t('network_adapter_hop_list') 内容每项 '{名称}: {跃点}'(v4) 关闭 t('close')；失败 snackError '错误'/t('get_hop_list_failed')

### ListenListPage（title t('listen_list')；action add → 添加对话框）
- 数据 playerState.listenList（默认 ['tcp://0.0.0.0:0','udp://0.0.0.0:0']）
- 空：dns/list_alt 空态 '暂无监听项' + 按钮 t('add_listen_item')
- 列表：ListView p16 Card(mb8) ListTile dns leading title=item trailing Row[edit(20,tooltip t('edit')) 4 delete(20,tooltip t('delete'))]
- 添加：AlertDialog t('add_listen_item') TextField autofocus label t('listen_item') hint 'localhost:8080'；t('cancel')/t('add')；trim 非空→addListen
- 编辑：t('edit_listen_item') 同款预填；t('cancel')/t('save')；非空且不同→updateListen(index)
- 删除确认：t('confirm_delete')/t('confirm_delete_listen_item',{item}) t('cancel')/t('delete')(red)→deleteListen(index)

### VpnSegmentPage（title t('custom_vpn_segment')；仅 Android 入口；action add）
- 空：vpn_lock 空态 'No VPN segments configured'(英文) + t('add_vpn_segment')
- 列表 Card(mb8) ListTile network_wifi trailing edit/delete
- 添加 t('add_vpn_segment')：TextField label t('vpn_segment_format_example') hint t('vpn_segment_input_hint')；t('cancel')/t('add')→sm.vpnState.addCustomVpn+persist
- 编辑 t('edit_vpn_segment')；删除 t('confirm_delete')/t('confirm_delete_vpn_segment',{vpn})

### PortWhitelistPage（工具页入口；title '端口白名单'）
- ListView p16 三卡（间16）：
  Card1 ListTile info_outline '白名单说明'/副标题三行 '配置允许通过的TCP/UDP端口。\n支持单个端口(80)和范围(8000-9000)。\n多个端口用逗号分隔，如： 80,443,8000-9000'
  Card2 ListTile storage 'TCP 端口白名单'/当前值（空→'未设置' bodySmall；非空→primary）trailing edit(tooltip '编辑')+非空时 clear(tooltip '清空')
  Card3 同 WiFi 'UDP 端口白名单'
- 编辑对话框 '编辑 {TCP|UDP} 端口白名单'：'格式说明：'(bold) 8 + '• 单个端口： 80' + '• 端口范围： 8000-9000' + '• 多个端口： 80,443,8000-9000' 16 + TextField(autofocus maxLines3 label '{type} 端口' hint '例如： 80,443,8000-9000' helper '留空表示不限制')；'取消'/'保存'→trim 保存（updateTcpWhitelist/updateUdpWhitelist）
- 清空确认 '确认清空'/'确定要清空 {type} 端口白名单吗？\n清空后将不限制端口访问。' '取消'/'清空'

## i18n 键使用
所有 t('key') 见 public/translations/zh.json（170 键扁平）。带参数：confirm_delete_listen_item{item}/confirm_delete_vpn_segment{vpn}/socks5_address_hint{port}/download_acceleration_auto_current{mirror}。

## 硬编码中文（保持原样）
'通用设置'/6 条副标题/'高级网络设置'/'日志'/'连接设置'/'配置连接重试行为'/'连接失败自动重试'/'连接失败时自动重新尝试连接'/'最大重试次数'/'当前设置为 {} 次'/'{} 次'/'减少动画更新'/'降低拓扑图与连线动画刷新频率，减少 GPU 占用'/'简约/详细/列表'/'重新下载'/'如果出现问题可以尝试重新下载！'/检查更新全套/'刷新'/'重试'/'暂无历史版本'/'加载失败'/'无法打开链接'/'暂无监听项'/'localhost:8080'/端口白名单全部文案

## 关键数值
列表 padding16；卡片间 16（主页组间 24、条目 8、日志 2、历史 12）；divider h1；空态 icon 64；小图标 20；Dropdown 容器宽 100；日志 12px monospace；日志上限 1000 条；SOCKS5 端口 1-65535；重试选项 [1,2,3,5,10]
URLs：历史版本 https://astral.fan/downloads.json；网盘 https://astral.fan/quick-start/download-install/；镜像 [https://gh.xmly.dev/,https://gh-proxy.org/,https://ghfast.top/]
