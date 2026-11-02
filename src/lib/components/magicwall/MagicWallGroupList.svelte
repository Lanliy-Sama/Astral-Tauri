<script module>
  /**
   * 规则描述拼装 —— 对应 magic_wall_group_list.dart 顶层 buildMagicWallRuleDescription。
   * [允许|阻止, TCP/UDP/BOTH/ANY 大写, (入站|出站), (从 IP), (端口 P)].join(' · ')
   */
  export function buildMagicWallRuleDescription(rule) {
    const parts = [];

    parts.push(rule.action === 'allow' ? '允许' : '阻止');
    parts.push((rule.protocol || '').toUpperCase());

    if (rule.direction !== 'both') {
      parts.push(rule.direction === 'inbound' ? '入站' : '出站');
    }

    if (rule.remoteIp != null) {
      parts.push(`从 ${rule.remoteIp}`);
    }

    if (rule.remotePort != null) {
      parts.push(`端口 ${rule.remotePort}`);
    }

    return parts.join(' · ');
  }
</script>

<script>
  /**
   * MagicWallGroupList —— 对应 magic_wall_group_list.dart。
   * 组卡列表 + 空状态（MagicWallEmptyState）+ 规则行（_MagicWallRuleTile）+
   * 底部“添加规则”按钮；组头/规则行 trailing 各带 PopupMenu。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import PopupMenu from '$lib/components/common/PopupMenu.svelte';

  let {
    groups = [],
    collapsedGroups = [],
    onToggleGroupCollapse = () => {},
    onToggleGroup = () => {},
    onEditGroup = () => {},
    onDeleteGroup = () => {},
    onAddRule = () => {},
    onToggleRule = () => {},
    onEditRule = () => {},
    onDeleteRule = () => {},
  } = $props();

  const RED = 'var(--flutter-red)';

  /* Icons.shield_outlined：Material Symbols 无该连字名，经 FILL 轴覆写为描边字形 */
  const SHIELD_OUTLINE =
    "font-variation-settings:'FILL' 0,'wght' 400,'GRAD' 0,'opsz' 24";

  const groupMenuItems = [
    { value: 'addRule', label: '添加规则', icon: 'add' },
    { value: 'edit', label: '编辑规则组', icon: 'edit' },
    { value: 'delete', label: '删除规则组', icon: 'delete', iconColor: RED, textColor: RED },
  ];
  const ruleMenuItems = [
    { value: 'edit', label: '编辑', icon: 'edit' },
    { value: 'delete', label: '删除', icon: 'delete', iconColor: RED, textColor: RED },
  ];

  /* PopupMenu 定位状态（菜单右缘对齐锚点右缘，向下展开）；
     弹出时附带全屏 barrier（对应 Flutter PopupRoute 的 ModalBarrier：吸收菜单外点击） */
  let groupMenu = $state({ open: false, x: 0, y: 0, bundle: null });
  let ruleMenu = $state({ open: false, x: 0, y: 0, rule: null });

  function openGroupMenu(bundle, e) {
    const rect = e.currentTarget.getBoundingClientRect();
    groupMenu = {
      open: true,
      x: window.innerWidth - rect.right,
      y: rect.bottom,
      bundle,
    };
  }

  function handleGroupMenuSelect(value) {
    const bundle = groupMenu.bundle;
    if (!bundle) return;
    if (value === 'addRule') {
      onAddRule(bundle.group.groupId, false);
    } else if (value === 'edit') {
      onEditGroup(bundle.group);
    } else if (value === 'delete') {
      onDeleteGroup(bundle.group);
    }
  }

  function openRuleMenu(rule, e) {
    const rect = e.currentTarget.getBoundingClientRect();
    ruleMenu = {
      open: true,
      x: window.innerWidth - rect.right,
      y: rect.bottom,
      rule,
    };
  }

  function handleRuleMenuSelect(value) {
    const rule = ruleMenu.rule;
    if (!rule) return;
    if (value === 'edit') {
      onEditRule(rule);
    } else if (value === 'delete') {
      onDeleteRule(rule);
    }
  }

  /* 规则头像底色：allow → (enabled && group.enabled ? green : grey)，block → red.shade400 */
  function ruleAvatarClass(rule, group) {
    if (rule.action === 'allow') {
      return rule.enabled && group.enabled ? 'allow-on' : 'allow-off';
    }
    return 'deny';
  }
</script>

{#if groups.length === 0}
  <!-- MagicWallEmptyState -->
  <div class="mw-empty">
    <Icon name="shield" size={80} color="#bdbdbd" style={SHIELD_OUTLINE} />
    <div class="mw-empty-title">还没有规则组</div>
    <div class="mw-empty-sub">点击右下角按钮创建规则组</div>
  </div>
{:else}
  <!-- ListView(padding: h12 v8) -->
  <div class="mw-list-view">
    {#each groups as bundle (bundle.group.groupId)}
      {@const isCollapsed = collapsedGroups.includes(bundle.group.groupId)}
      {@const enabledCount = bundle.rules.filter((r) => r.enabled).length}
      <!-- 组卡 Card(margin: v8) -->
      <div class="md-card mw-group-card">
        <!-- 组头 ListTile（onTap 切折叠） -->
        <div
          class="mw-tile mw-group-header"
          role="button"
          tabindex="0"
          onclick={() => onToggleGroupCollapse(bundle.group.groupId)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') onToggleGroupCollapse(bundle.group.groupId);
          }}
        >
          <div class="mw-avatar" class:enabled={bundle.group.enabled}>
            <Icon
              name={bundle.group.enabled ? 'layers' : 'layers_outlined'}
              size={24}
              color="#ffffff"
            />
          </div>
          <div class="mw-tile-content">
            <div class="mw-group-name" class:off={!bundle.group.enabled}>
              {bundle.group.name}
            </div>
            <div class="mw-group-sub">
              <div class="mw-sub-line">
                进程: {bundle.group.processName ? bundle.group.processName : '未绑定'}
              </div>
              <div class="mw-sub-line mw-sub-12">
                规则: {bundle.rules.length} 项 · 已启用 {enabledCount} 项
              </div>
              {#if bundle.group.autoManage}
                <div class="mw-sub-line mw-sub-12 mw-auto">自动监听进程</div>
              {/if}
            </div>
          </div>
          <!-- trailing Row(min)：折叠按钮 + Switch + PopupMenuButton -->
          <div class="mw-tile-trailing" onclick={(e) => e.stopPropagation()}>
            <button
              class="md-icon-btn"
              onclick={() => onToggleGroupCollapse(bundle.group.groupId)}
            >
              <Icon name={isCollapsed ? 'expand_more' : 'expand_less'} size={24} />
            </button>
            <Switch
              value={bundle.group.enabled}
              onChange={() => onToggleGroup(bundle.group)}
            />
            <button
              class="md-icon-btn"
              onclick={(e) => openGroupMenu(bundle, e)}
              aria-label="更多操作"
            >
              <Icon name="more_vert" size={24} />
            </button>
          </div>
        </div>

        {#if !isCollapsed && bundle.rules.length}
          <div class="md-divider"></div>
        {/if}

        {#if !isCollapsed}
          {#each bundle.rules as rule, i (rule.ruleId)}
            <!-- _MagicWallRuleTile（isThreeLine, contentPadding l72 r16） -->
            <div class="mw-tile mw-rule-tile">
              <div class="mw-avatar mw-rule-avatar {ruleAvatarClass(rule, bundle.group)}">
                <Icon
                  name={rule.action === 'allow' ? 'check' : 'block'}
                  size={24}
                  color="#ffffff"
                />
              </div>
              <div class="mw-tile-content">
                <div class="mw-rule-name" class:off={!rule.enabled}>
                  {rule.name}
                </div>
                <div class="mw-rule-sub">
                  <div class="mw-sub-line">{buildMagicWallRuleDescription(rule)}</div>
                  {#if rule.description}
                    <div class="mw-sub-line mw-sub-12 mw-note">{rule.description}</div>
                  {/if}
                </div>
              </div>
              <div class="mw-tile-trailing">
                <Switch value={rule.enabled} onChange={() => onToggleRule(rule)} />
                <button
                  class="md-icon-btn"
                  onclick={(e) => openRuleMenu(rule, e)}
                  aria-label="更多操作"
                >
                  <Icon name="more_vert" size={24} />
                </button>
              </div>
            </div>
            {#if i !== bundle.rules.length - 1}
              <!-- Divider(indent: 72, endIndent: 16, height: 1) -->
              <div class="mw-rule-divider"></div>
            {/if}
          {/each}
          <!-- Align(left) Padding(72,8,16,12) TextButton.icon(add, '添加规则') -->
          <div class="mw-add-rule-row">
            <button
              class="md-btn md-text-btn"
              onclick={() => onAddRule(bundle.group.groupId, false)}
            >
              <Icon name="add" size={18} />
              添加规则
            </button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<!-- 菜单外点击 barrier（z 1100，低于 .md-menu 的 1200）：吸收点击并关闭菜单 -->
{#if groupMenu.open}
  <div class="mw-menu-barrier" onclick={() => (groupMenu.open = false)}></div>
{/if}
<!-- 组 PopupMenu -->
<PopupMenu
  bind:open={groupMenu.open}
  x={groupMenu.x}
  y={groupMenu.y}
  alignEnd
  items={groupMenuItems}
  itemIconSize={20}
  onSelect={handleGroupMenuSelect}
/>
{#if ruleMenu.open}
  <div class="mw-menu-barrier" onclick={() => (ruleMenu.open = false)}></div>
{/if}
<!-- 规则 PopupMenu -->
<PopupMenu
  bind:open={ruleMenu.open}
  x={ruleMenu.x}
  y={ruleMenu.y}
  alignEnd
  items={ruleMenuItems}
  itemIconSize={20}
  onSelect={handleRuleMenuSelect}
/>

<style>
  /* ---------- 空状态（Center Column） ---------- */
  .mw-empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .mw-empty-title {
    margin-top: 16px;
    font-size: 18px;
    line-height: 24px;
    color: #757575; /* Colors.grey[600] */
  }
  .mw-empty-sub {
    margin-top: 8px;
    font-size: 14px;
    line-height: 20px;
    color: var(--flutter-grey); /* Colors.grey[500] */
  }

  /* ---------- 列表 ---------- */
  .mw-list-view {
    padding: 8px 12px; /* EdgeInsets.symmetric(horizontal: 12, vertical: 8) */
  }
  .mw-group-card {
    margin: 8px 0; /* EdgeInsets.symmetric(vertical: 8) */
    overflow: hidden;
  }

  /* ---------- ListTile 基础（gap 16 / padding 8 16 / min 56） ---------- */
  .mw-tile {
    display: flex;
    gap: 16px;
    padding: 8px 16px;
    position: relative;
  }
  .mw-group-header {
    align-items: center;
    min-height: 56px;
    cursor: pointer;
  }
  .mw-group-header:hover {
    background: var(--md-on-surface-08);
  }
  .mw-tile-content {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }
  .mw-tile-trailing {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  /* ---------- CircleAvatar（40×40） ---------- */
  .mw-avatar {
    width: 40px;
    height: 40px;
    min-width: 40px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--flutter-grey); /* Colors.grey */
  }
  .mw-avatar.enabled {
    background: var(--flutter-blue); /* Colors.blue */
  }
  .mw-rule-avatar.allow-on {
    background: var(--flutter-green); /* Colors.green */
  }
  .mw-rule-avatar.allow-off {
    background: var(--flutter-grey); /* Colors.grey */
  }
  .mw-rule-avatar.deny {
    background: var(--flutter-red-400); /* Colors.red.shade400 */
  }

  /* ---------- 组头文字 ---------- */
  .mw-group-name {
    font-size: 16px;
    line-height: 24px;
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .mw-group-name.off {
    color: var(--flutter-grey); /* 组禁用 → grey */
  }
  .mw-group-sub {
    margin-top: 4px; /* subtitle Column 顶部 SizedBox(height: 4) */
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .mw-sub-line {
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-variant);
  }
  .mw-sub-12 {
    font-size: 12px;
    line-height: 16px;
  }
  .mw-auto {
    color: #607d8b; /* Colors.blueGrey */
  }

  /* ---------- 规则行（isThreeLine + contentPadding l72 r16） ---------- */
  .mw-rule-tile {
    align-items: flex-start;
    min-height: 88px; /* isThreeLine */
    padding: 8px 16px 8px 72px;
  }
  .mw-rule-tile .mw-tile-content {
    justify-content: flex-start;
  }
  .mw-rule-name {
    font-size: 16px;
    line-height: 24px;
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .mw-rule-name.off {
    color: var(--flutter-grey); /* 规则禁用 → grey */
  }
  .mw-rule-sub {
    margin-top: 4px; /* subtitle Column 顶部 SizedBox(height: 4) */
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .mw-note {
    margin-top: 2px; /* SizedBox(height: 2) */
    font-style: italic;
  }
  /* Divider(indent: 72, endIndent: 16) */
  .mw-rule-divider {
    height: 1px;
    margin-left: 72px;
    margin-right: 16px;
    background: var(--md-outline-variant);
    flex-shrink: 0;
  }

  /* ---------- 底部添加规则（Align left + Padding(72,8,16,12)） ---------- */
  .mw-add-rule-row {
    padding: 8px 16px 12px 72px;
    display: flex;
    justify-content: flex-start;
  }

  /* ---------- PopupMenu barrier（对应 Flutter ModalBarrier） ---------- */
  .mw-menu-barrier {
    position: fixed;
    inset: 0;
    z-index: 1100; /* .md-menu 为 1200，菜单浮于 barrier 之上 */
  }
</style>
