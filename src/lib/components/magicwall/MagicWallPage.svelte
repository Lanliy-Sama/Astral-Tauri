<script>
  /**
   * MagicWallPage —— 对应 magic_wall_page.dart（魔法墙主页面）。
   * AppBar 状态指示器（shield + 条数）+ 控制面板 + 组列表 + FAB 创建菜单；
   * initState：loadData → fixIncompleteAppPaths、checkStatus、Windows 启动进程监控。
   */
  import { onMount } from 'svelte';
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import { snackError } from '$lib/components/common/snackbars.svelte.js';
  import { isWindows } from '$lib/utils/platform.js';
  import { mwController } from '$lib/services/magic-wall-service.svelte.js';
  import MagicWallControlPanel from './MagicWallControlPanel.svelte';
  import MagicWallGroupList from './MagicWallGroupList.svelte';
  import BottomSheetMenu from './BottomSheetMenu.svelte';
  import MagicWallGroupDialog from './dialogs/MagicWallGroupDialog.svelte';
  import MagicWallRuleDialog from './dialogs/MagicWallRuleDialog.svelte';

  /* Icons.shield_outlined：Material Symbols 无该连字名，经 FILL 轴覆写为描边字形 */
  const SHIELD_OUTLINE =
    "font-variation-settings:'FILL' 0,'wght' 400,'GRAD' 0,'opsz' 24";

  /* 本地折叠状态（对应 _collapsedGroups Set<String>） */
  let collapsedGroups = $state([]);

  /* 弹层状态 */
  let sheetOpen = $state(false);
  let groupDialog = $state({ open: false, group: null });
  let ruleDialog = $state({
    open: false,
    rule: null,
    selectedGroupId: null,
    allowGroupChange: true,
  });
  let deleteDialog = $state({
    open: false,
    kind: 'rule',
    name: '',
    rule: null,
    group: null,
  });

  let monitorStarted = false;

  onMount(() => {
    // onValidGroupIdsChanged → 清理已失效的折叠状态
    mwController.onValidGroupIdsChanged = (validGroupIds) => {
      const next = collapsedGroups.filter((id) => validGroupIds.has(id));
      if (next.length !== collapsedGroups.length) collapsedGroups = next;
    };
    mwController.loadData().then(() => mwController.fixIncompleteAppPaths());
    mwController.checkStatus();
    if (isWindows()) {
      mwController.startProcessMonitor();
      monitorStarted = true;
    }
    return () => {
      mwController.onValidGroupIdsChanged = null;
      if (monitorStarted) mwController.stopProcessMonitor();
    };
  });

  /* ---------------- 折叠 ---------------- */

  function toggleGroupCollapse(groupId) {
    collapsedGroups = collapsedGroups.includes(groupId)
      ? collapsedGroups.filter((id) => id !== groupId)
      : [...collapsedGroups, groupId];
  }

  /* ---------------- 规则动作 ---------------- */

  function addRule(groupId = null, allowGroupChange = true) {
    const groups = mwController.groups.map((bundle) => bundle.group);
    if (groups.length === 0) {
      snackError('错误', '请先创建规则组');
      return;
    }

    const targetGroupId = groupId ?? groups[0].groupId;

    ruleDialog = {
      open: true,
      rule: null,
      selectedGroupId: targetGroupId,
      allowGroupChange,
    };
  }

  function editRule(rule) {
    const groups = mwController.groups.map((bundle) => bundle.group);
    if (groups.length === 0) {
      snackError('错误', '请先创建规则组');
      return;
    }

    ruleDialog = {
      open: true,
      rule,
      selectedGroupId: rule.groupId,
      allowGroupChange: true,
    };
  }

  function requestDeleteRule(rule) {
    deleteDialog = {
      open: true,
      kind: 'rule',
      name: rule.name,
      rule,
      group: null,
    };
  }

  function handleRuleConfirm(rule) {
    const ctx = ruleDialog;
    ruleDialog.open = false;
    const groupsList = mwController.groups.map((bundle) => bundle.group);
    if (ctx.rule) {
      mwController.updateRule(rule, { groupsList });
    } else {
      mwController.addRule(rule, {
        targetGroupId: ctx.selectedGroupId,
        groupsList,
      });
    }
  }

  /* ---------------- 规则组动作 ---------------- */

  function addGroup() {
    groupDialog = { open: true, group: null };
  }

  function editGroup(group) {
    groupDialog = { open: true, group };
  }

  function requestDeleteGroup(group) {
    deleteDialog = {
      open: true,
      kind: 'group',
      name: group.name,
      rule: null,
      group,
    };
  }

  function handleGroupConfirm(group) {
    const editing = !!groupDialog.group;
    groupDialog.open = false;
    if (editing) {
      mwController.updateGroup(group);
    } else {
      mwController.addGroup(group);
    }
  }

  /* ---------------- 删除确认 ---------------- */

  async function confirmDelete() {
    const target = deleteDialog;
    deleteDialog.open = false;
    if (target.kind === 'rule') {
      await mwController.deleteRule(target.rule);
    } else {
      await mwController.deleteGroup(target.group);
    }
  }

  /* ---------------- FAB 创建菜单 ---------------- */

  /* showModalBottomSheet：'添加规则组' / '添加规则'（groups 空时禁用） */
  const sheetItems = $derived([
    { icon: 'layers', label: '添加规则组', disabled: false, onSelect: () => addGroup() },
    {
      icon: 'playlist_add',
      label: '添加规则',
      disabled: mwController.groups.length === 0,
      onSelect: () => addRule(),
    },
  ]);
</script>

<SubPage title="魔法墙" scrollable={false}>
  {#snippet actions()}
    <!-- 状态指示器：Padding(h16) Row(min)[Icon(shield, 20) 8 Text(14)] -->
    <div class="mw-status">
      <Icon
        name="shield"
        size={20}
        color={mwController.isRunning ? 'var(--flutter-green)' : 'var(--flutter-grey)'}
        style={mwController.isRunning ? '' : SHIELD_OUTLINE}
      />
      <span class="mw-status-text">{mwController.activeRulesCount} 条规则</span>
    </div>
  {/snippet}

  <div class="mw-root">
    <MagicWallControlPanel
      isRunning={mwController.isRunning}
      onToggle={() => mwController.toggleEngine()}
    />
    <div class="md-divider"></div>
    <div class="mw-list-area">
      <MagicWallGroupList
        groups={mwController.groups}
        {collapsedGroups}
        onToggleGroupCollapse={toggleGroupCollapse}
        onToggleGroup={(group) => mwController.toggleGroup(group)}
        onEditGroup={editGroup}
        onDeleteGroup={requestDeleteGroup}
        onAddRule={addRule}
        onToggleRule={(rule) => mwController.toggleRule(rule)}
        onEditRule={editRule}
        onDeleteRule={requestDeleteRule}
      />
    </div>
    <!-- Scaffold floatingActionButton -->
    <button class="md-fab mw-fab" onclick={() => (sheetOpen = true)} aria-label="添加">
      <Icon name="add" size={24} />
    </button>
  </div>
</SubPage>

<!-- FAB 创建菜单（showModalBottomSheet） -->
<BottomSheetMenu bind:open={sheetOpen} items={sheetItems} />

<!-- 规则组对话框 -->
{#if groupDialog.open}
  <MagicWallGroupDialog
    group={groupDialog.group}
    onConfirm={handleGroupConfirm}
    onCancel={() => (groupDialog.open = false)}
  />
{/if}

<!-- 规则对话框 -->
{#if ruleDialog.open}
  <MagicWallRuleDialog
    rule={ruleDialog.rule}
    groups={mwController.groups.map((bundle) => bundle.group)}
    selectedGroupId={ruleDialog.selectedGroupId}
    allowGroupChange={ruleDialog.allowGroupChange}
    onConfirm={handleRuleConfirm}
    onCancel={() => (ruleDialog.open = false)}
  />
{/if}

<!-- 确认删除对话框 -->
<Dialog bind:open={deleteDialog.open} title="确认删除">
  <div>{`确定要删除${deleteDialog.kind === 'rule' ? '规则' : '规则组'} "${deleteDialog.name}" 吗？`}</div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (deleteDialog.open = false)}>取消</button>
    <button class="md-btn md-text-btn" onclick={confirmDelete}>删除</button>
  {/snippet}
</Dialog>

<style>
  /* Scaffold body：Column[ControlPanel, Divider(h1), Expanded(list)] */
  .mw-root {
    height: 100%;
    position: relative;
    display: flex;
    flex-direction: column;
  }
  .mw-list-area {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  /* FAB：右下角固定 */
  .mw-fab {
    position: absolute;
    right: 16px;
    bottom: 16px;
    z-index: 5;
  }
  /* AppBar 状态指示器 */
  .mw-status {
    padding: 0 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .mw-status-text {
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface);
    white-space: nowrap;
  }
</style>
