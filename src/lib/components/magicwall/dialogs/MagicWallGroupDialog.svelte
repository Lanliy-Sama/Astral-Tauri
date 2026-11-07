<script>
  /**
   * MagicWallGroupDialog —— 对应 magic_wall_group_dialog.dart（规则组编辑对话框）。
   * 由父级以 {#if open} 条件挂载（每次打开重建表单状态，对应 StatefulWidget initState）。
   */
  import Dialog from '$lib/components/common/Dialog.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import { snackError } from '$lib/components/common/SnackbarStore.svelte';
  import { uuidV4 } from '$lib/utils/Display.svelte';

  let {
    group = null,
    onConfirm = () => {},
    onCancel = () => {},
  } = $props();

  let open = $state(true);
  let name = $state(group?.name ?? '');
  let processName = $state(group?.processName ?? '');
  let autoManage = $state(group?.autoManage ?? true);
  let enabled = $state(group?.enabled ?? false);

  function cancel() {
    open = false;
    onCancel?.();
  }

  function save() {
    const trimmedName = name.trim();
    const trimmedProcess = processName.trim();

    if (!trimmedName) {
      snackError('无法保存', '请输入规则组名称');
      return;
    }

    if (autoManage && !trimmedProcess) {
      snackError('无法保存', '启用自动监听时需填写进程名称');
      return;
    }

    const now = Date.now();
    onConfirm({
      id: group?.id ?? null, // 主键由存储层自增分配
      groupId: group?.groupId ?? uuidV4(),
      name: trimmedName,
      processName: trimmedProcess,
      autoManage,
      enabled,
      createdAt: group?.createdAt ?? now,
      updatedAt: now,
    });
  }
</script>

<Dialog bind:open={open} title={group ? '编辑规则组' : '添加规则组'} width={420} onDismiss={cancel}>
  <!-- SingleChildScrollView + SizedBox(width: 420) + Column -->
  <div class="mw-form">
    <TextField bind:value={name} label="规则组名称 *" prefixIcon="layers" />
    <div class="gap-16"></div>
    <TextField
      bind:value={processName}
      label="绑定进程名称"
      hint="如: game.exe"
      prefixIcon="memory"
    />
    <div class="gap-16"></div>
    <div
      class="md-switch-list-tile"
      onclick={() => (autoManage = !autoManage)}
    >
      <div class="tile-content">
        <div class="tile-title">自动监听进程</div>
        <div class="tile-subtitle">进程启动启用规则组, 进程结束关闭规则组</div>
      </div>
      <span class="mw-switch-wrap" onclick={(e) => e.stopPropagation()}>
        <Switch bind:value={autoManage} />
      </span>
    </div>
    <div
      class="md-switch-list-tile"
      onclick={() => (enabled = !enabled)}
    >
      <div class="tile-content">
        <div class="tile-title">启用规则组</div>
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
  .mw-form {
    display: flex;
    flex-direction: column;
  }
  /* SizedBox(height: 16) */
  .gap-16 {
    height: 16px;
    flex-shrink: 0;
  }
  /* SwitchListTile 内点击 Switch 时不冒泡到 tile（避免双翻转） */
  .mw-switch-wrap {
    display: inline-flex;
    flex-shrink: 0;
  }
</style>
