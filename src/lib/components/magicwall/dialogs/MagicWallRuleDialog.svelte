<script>
  /**
   * MagicWallRuleDialog —— 对应 magic_wall_rule_dialog.dart（规则编辑对话框）。
   * 由父级以 {#if open} 条件挂载（每次打开重建表单状态）。
   * 忠实保留原版行为：对话框不编辑 description/保存时置空（原 Dart 亦未赋值该字段）。
   */
  import Dialog from '$lib/components/common/Dialog.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import DropdownField from '$lib/components/common/DropdownField.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import { snackError } from '$lib/components/common/snackbars.svelte.js';
  import { uuidV4 } from '$lib/utils/display.js';

  let {
    rule = null,
    groups = [],
    selectedGroupId = null,
    allowGroupChange = true,
    onConfirm = () => {},
    onCancel = () => {},
  } = $props();

  let open = $state(true);
  let name = $state(rule?.name ?? '');
  let remoteIp = $state(rule?.remoteIp ?? '');
  let localIp = $state(rule?.localIp ?? '');
  let remotePort = $state(rule?.remotePort ?? '');
  let localPort = $state(rule?.localPort ?? '');

  let action = $state(rule?.action ?? 'block');
  let protocol = $state(rule?.protocol ?? 'both');
  let direction = $state(rule?.direction ?? 'both');
  let enabled = $state(rule?.enabled ?? true);
  let groupId = $state(
    rule?.groupId ??
      selectedGroupId ??
      (groups.length ? groups[0].groupId : ''),
  );

  const groupOptions = groups.map((g) => ({ value: g.groupId, label: g.name }));
  const actionOptions = [
    { value: 'allow', label: '允许' },
    { value: 'block', label: '阻止' },
  ];
  const protocolOptions = [
    { value: 'tcp', label: 'TCP' },
    { value: 'udp', label: 'UDP' },
    { value: 'both', label: 'TCP+UDP' },
    { value: 'any', label: '任意' },
  ];
  const directionOptions = [
    { value: 'inbound', label: '⬇️ 入站' },
    { value: 'outbound', label: '⬆️ 出站' },
    { value: 'both', label: '↕️ 双向' },
  ];

  function cancel() {
    open = false;
    onCancel?.();
  }

  function save() {
    if (!name.trim()) {
      snackError('无法保存', '请输入规则名称');
      return;
    }

    if (!groupId) {
      snackError('无法保存', '请选择规则组');
      return;
    }

    const now = Date.now();
    const existingAppPath = rule?.appPath ?? null;
    const trimOrNull = (v) => (v.trim() ? v.trim() : null);

    onConfirm({
      id: rule?.id ?? null, // 主键由存储层自增分配
      ruleId: rule?.ruleId ?? uuidV4(),
      groupId,
      name: name.trim(),
      enabled,
      action,
      protocol,
      direction,
      appPath: existingAppPath ? existingAppPath : null,
      remoteIp: trimOrNull(remoteIp),
      localIp: trimOrNull(localIp),
      remotePort: trimOrNull(remotePort),
      localPort: trimOrNull(localPort),
      description: null, // 原版对话框未提供描述编辑，保存即置空
      createdAt: rule?.createdAt ?? now,
      updatedAt: now,
      priority: rule?.priority ?? 0,
    });
  }
</script>

<Dialog bind:open={open} title={rule ? '编辑规则' : '添加规则'} width={500} onDismiss={cancel}>
  <!-- SingleChildScrollView + SizedBox(width: 500) + Column -->
  <div class="mw-rule-form">
    {#if groups.length}
      <!-- DropdownButtonFormField '所属规则组'（prefixIcon: layers，allowGroupChange 才可改） -->
      <div class="dd-icon-field">
        <span class="dd-prefix-icon"><Icon name="layers" size={24} /></span>
        <DropdownField
          bind:value={groupId}
          options={groupOptions}
          label="所属规则组"
          disabled={!allowGroupChange}
          style="padding-left:36px"
        />
      </div>
      <div class="gap-16"></div>
    {/if}

    <!-- 规则名称 -->
    <TextField bind:value={name} label="规则名称 *" prefixIcon="label" />
    <div class="gap-16"></div>

    <!-- 动作 + 协议 -->
    <div class="mw-row">
      <div class="mw-field">
        <DropdownField bind:value={action} options={actionOptions} label="动作" />
      </div>
      <div class="mw-field">
        <DropdownField bind:value={protocol} options={protocolOptions} label="协议" />
      </div>
    </div>
    <div class="gap-16"></div>

    <!-- 方向（prefixIcon: swap_horiz） -->
    <div class="dd-icon-field">
      <span class="dd-prefix-icon"><Icon name="swap_horiz" size={24} /></span>
      <DropdownField
        bind:value={direction}
        options={directionOptions}
        label="方向"
        style="padding-left:36px"
      />
    </div>
    <div class="gap-16"></div>

    <!-- 远程配置 -->
    <div class="mw-row">
      <div class="mw-field">
        <TextField
          bind:value={remoteIp}
          label="远程 IP（可选）"
          hint="192.168.1.1 或 192.168.0.0/16"
        />
      </div>
      <div class="mw-field">
        <TextField
          bind:value={remotePort}
          label="远程端口（可选）"
          hint="80 或 8000-9000"
          inputmode="numeric"
        />
      </div>
    </div>
    <div class="gap-16"></div>

    <!-- 本地配置 -->
    <div class="mw-row">
      <div class="mw-field">
        <TextField
          bind:value={localIp}
          label="本地 IP（可选）"
          hint="192.168.1.1 或 192.168.0.0/16"
        />
      </div>
      <div class="mw-field">
        <TextField
          bind:value={localPort}
          label="本地端口（可选）"
          hint="80 或 8000-9000"
          inputmode="numeric"
        />
      </div>
    </div>
    <div class="gap-16"></div>

    <div class="md-switch-list-tile" onclick={() => (enabled = !enabled)}>
      <div class="tile-content">
        <div class="tile-title">启用规则</div>
      </div>
      <span class="mw-switch-wrap" onclick={(e) => e.stopPropagation()}>
        <Switch bind:value={enabled} />
      </span>
    </div>
  </div>

  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={cancel}>取消</button>
    <button class="md-btn md-elevated-btn" onclick={save}>保存</button>
  {/snippet}
</Dialog>

<style>
  .mw-rule-form {
    display: flex;
    flex-direction: column;
  }
  /* SizedBox(height: 16) */
  .gap-16 {
    height: 16px;
    flex-shrink: 0;
  }
  /* Row + SizedBox(width: 8) + Expanded×2 */
  .mw-row {
    display: flex;
    gap: 8px;
  }
  .mw-field {
    flex: 1 1 0;
    min-width: 0;
  }
  /* prefixIcon（对应 InputDecoration.prefixIcon）：绝对定位在框内左侧 */
  .dd-icon-field {
    position: relative;
  }
  .dd-prefix-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--md-on-surface-variant);
    pointer-events: none;
    z-index: 2;
    display: inline-flex;
  }
  /* SwitchListTile 内点击 Switch 时不冒泡到 tile（避免双翻转） */
  .mw-switch-wrap {
    display: inline-flex;
    flex-shrink: 0;
  }
</style>
