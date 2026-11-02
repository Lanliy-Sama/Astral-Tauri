# MagicWall 模块规格（源码核对版）

## MagicWallPage（从工具页进入，AppBar '魔法墙'）
- AppBar actions：状态指示器 Padding(h16) Row[min]：Icon(shield|shield_outlined, 运行中?green:grey, 20) + 8 + '${activeRulesCount} 条规则'(14)
- body Column：MagicWallControlPanel + Divider(h1) + Expanded 组列表
- FAB add → showModalBottomSheet(SafeArea+Wrap)：ListTile layers '添加规则组'；ListTile playlist_add '添加规则'（groups 空时禁用）
- initState：controller.init + loadData().then(fixIncompleteAppPaths) + checkStatus；Windows 启动 5s 进程监控轮询
- 本地状态：_collapsedGroups Set（折叠组）

## ControlPanel
Card(margin 16) Padding(16) Column(start)：
- Row：Expanded Column['魔法墙引擎'(18,bold) 4 状态文字(isRunning?'运行中':'已停止'，色 green/grey,14)] + Switch(isRunning, onChanged 非 Windows null)
- 非 Windows：8 + '⚠️ 魔法墙仅支持 Windows 平台'(12, orange)

## 组列表（groups 空 → 空状态）
- 空状态：Center Column[shield_outlined(80,grey[400]) 16 '还没有规则组'(18,grey[600]) 8 '点击右下角按钮创建规则组'(14,grey[500])]
- ListView(padding h12v8)
- 组卡 Card(margin v8) Column：
  - 头 ListTile（onTap 切折叠）：leading CircleAvatar(bg enabled?blue:grey, icon layers/layers_outlined 白)；title 组名(bold, 禁用 grey)；subtitle Column['进程: ${processName||'未绑定'}' 4 '规则: ${n} 项 · 已启用 ${m} 项'(12) (autoManage:'自动监听进程'(12,blueGrey))]；trailing Row[IconButton(expand_more/expand_less) Switch(enabled→onToggleGroup) PopupMenu('addRule':add20+'添加规则'；'edit':edit20+'编辑规则组'；'delete':delete20 red+'删除规则组' red)]
  - 展开：rules 非空 Divider(h1)；每条 _MagicWallRuleTile 间 Divider(indent72,endIndent16,h1)；底部 Align 左 Padding(72,8,16,12) TextButton.icon(add,'添加规则')（allowGroupChange:false）
- 规则行 ListTile(isThreeLine, contentPadding l72 r16)：leading CircleAvatar(bg action=='allow'?(enabled&&group.enabled?green:grey):red.shade400, icon check/block 白)；title 规则名(bold, 禁用 grey)；subtitle Column[4 buildMagicWallRuleDescription 2 (description 非空: description 12 italic)]；trailing Row[Switch(rule.enabled) PopupMenu('edit' '编辑'/'delete' '删除' red)]
- 规则描述：parts = [action=='allow'?'允许':'阻止', protocol.toUpperCase()] + (direction!='both' ? (inbound?'入站':'出站')) + (remoteIp?'从 ${remoteIp}') + (remotePort?'端口 ${remotePort}')，join(' · ')

## 确认删除对话框
'确认删除' / '确定要删除规则 "${name}" 吗？' 或 '确定要删除规则组 "${name}" 吗？'；'取消'/'删除'

## 组对话框（MagicWallGroupDialog）
AlertDialog '添加规则组'/'编辑规则组'；SingleChildScrollView+SizedBox(420)+Column：TextField '规则组名称 *'(prefixIcon layers) 16 TextField '绑定进程名称'(hint '如: game.exe', prefixIcon memory) 16 SwitchListTile '自动监听进程'(subtitle '进程启动启用规则组, 进程结束关闭规则组') SwitchListTile '启用规则组'；actions '取消'/'保存'
初始：autoManage=group?.autoManage ?? true；enabled=group?.enabled ?? false
校验：name 空 → snackError '无法保存'/'请输入规则组名称'；autoManage&&process 空 → '无法保存'/'启用自动监听时需填写进程名称'
保存：groupId 保留或 uuid；createdAt 保留或 now；updatedAt=now

## 规则对话框（MagicWallRuleDialog）
AlertDialog '添加规则'/'编辑规则'；SizedBox(500)：(groups 非空) DropdownButtonFormField '所属规则组'(prefixIcon layers, allowGroupChange 才可改) 16 TextField '规则名称 *'(prefixIcon label) 16 Row[动作 Dropdown(allow 允许/block 阻止) 8 协议 Dropdown(tcp TCP/udp UDP/both TCP+UDP/any 任意)] 16 方向 Dropdown(prefixIcon swap_horiz; inbound '⬇️ 入站'/outbound '⬆️ 出站'/both '↕️ 双向') 16 Row[TextField '远程 IP（可选）'(hint '192.168.1.1 或 192.168.0.0/16') 8 TextField '远程端口（可选）'(hint '80 或 8000-9000', number)] 16 Row[本地 IP + 本地端口（同上）] 16 SwitchListTile '启用规则'；actions '取消'/'保存'
初始：action='block' protocol='both' direction='both' enabled=true；_groupId=rule?.groupId ?? selectedGroupId ?? groups.first
校验：名称空 → '无法保存'/'请输入规则名称'；_groupId 空 → '无法保存'/'请选择规则组'
保存：ruleId 保留或 uuid；可选文本 trim 空→null；appPath 保留原值；priority 保留或 0

## 控制器行为（magic_wall_controller）
- signals：isRunning / groups(bundles) / activeRulesCount = Σ(enabled 组内 enabled 规则)
- toggleEngine：运行→stop+recordEvent('off','手动操作')+snack '魔法墙已停止'；停止→startAndSyncRules→isRunning=true+recordEvent('on')+snack '魔法墙已启动'；异常→snackError '操作失败: ${e}' + checkStatus 刷新
- CRUD 全部 loadData 全量重读（rules priority 降序，groups name 升序）：
  addRule：解析 groupId → rule.appPath=组路径 → dao.addRule → loadData → engine 同步；成功 '规则已添加' 失败 '添加规则失败: ${e}'
  updateRule / deleteRule(先引擎移除再删库) / toggleRule / toggleGroup / addGroup('规则组已添加') / updateGroup / deleteGroup(先引擎移除)
- 引擎同步：仅 isRunning && 组enabled && 规则enabled 时 addMagicWallRule；update=remove+add；stop 清空 Rust 规则库 → start 时重推全部
- 进程监控（仅 Windows，5s）：autoManage 组 → getProcessExecutablePath(processName)；进程出现：引擎未运行→自动启动+auto_on 事件；组未启用→启用；推送规则；进程消失：移除规则+禁用组+无其他启用组时自动停止引擎
- 事件日志 targetType engine/group，action on/off/auto_on/auto_off

## 数据模型
- Rule：ruleId(uuid,unique) groupId name enabled(true) action('block') protocol('both') direction('both') appPath/remoteIp/localIp/remotePort/localPort/description(可空) createdAt/updatedAt(毫秒) priority(0)
- Group：groupId(unique) name processName enabled(false) autoManage(true) createdAt/updatedAt
- 存储：db 表 mw_rules / mw_groups / mw_events（id+data JSON）

## Rust API（rust/api.js 已有）
startMagicWall/stopMagicWall/addMagicWallRule/removeMagicWallRule/updateMagicWallRule/getMagicWallStatus({isRunning,activeRules,totalRules})
错误文案（透传 snackbar）：'魔法墙已经在运行'/'魔法墙未运行'/'规则不存在'/'魔法墙仅支持 Windows 平台'

## 控制器服务实现要点（需新建 magic-wall-service.svelte.js）
- groups 排序：rules priority 降序，groups name 升序；孤儿规则迁移（groupId 为空的规则→新建组'导入规则组'，重名 baseName(2)(3)...）
- 浏览器模式 processMonitor 直接不启用（isWindows() false）

## Snackbar 标题固定：成功 '成功'，错误 '错误'
