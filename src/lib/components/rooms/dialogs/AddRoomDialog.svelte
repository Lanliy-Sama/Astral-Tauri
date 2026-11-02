<script>
  /**
   * 添加房间对话框 —— 翻译自 features/rooms/dialogs/add_room_dialog.dart。
   * 确定后走 addEncryptedRoom：加密房间 roomName/password/messageKey = UUID v4。
   */
  import TextField from '$lib/components/common/TextField.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { randomRoomName, uuidV4 } from '$lib/utils/display.js';

  let { open = $bindable(false) } = $props();

  let name = $state('');
  let isEncrypted = $state(true);
  let roomName = $state('');
  let roomPassword = $state('');

  /* 每次打开重置（对应 showAddRoomDialog 的局部变量初始值） */
  $effect(() => {
    if (open) {
      name = randomRoomName();
      isEncrypted = true;
      roomName = '';
      roomPassword = '';
    }
  });

  /* addEncryptedRoom（room_page.dart 中的顶层函数） */
  async function confirmAdd() {
    await sm.addRoom({
      id: null,
      name,
      encrypted: isEncrypted,
      roomName: isEncrypted ? uuidV4() : roomName,
      password: isEncrypted ? uuidV4() : roomPassword,
      messageKey: isEncrypted ? uuidV4() : '',
      tags: [],
      servers: [],
      customParam: '',
      networkConfigJson: '',
      sortOrder: 0,
    });
    open = false;
  }
</script>

<Dialog bind:open={open} title="添加房间">
  <div class="dialog-body">
    <TextField bind:value={name} label="房间名称" />
    <div class="gap-8"></div>
    <!-- SwitchListTile 是否保护 -->
    <div
      class="md-switch-list-tile switch-tile"
      onclick={() => (isEncrypted = !isEncrypted)}
    >
      <div class="tile-content">
        <div class="tile-title">是否保护</div>
      </div>
      <span class="switch-host" onclick={(e) => e.stopPropagation()}>
        <Switch bind:value={isEncrypted} />
      </span>
    </div>

    {#if !isEncrypted}
      <TextField bind:value={roomName} label="房间号" />
      <div class="gap-8"></div>
      <TextField bind:value={roomPassword} label="房间密码" />
    {/if}
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (open = false)}>取消</button>
    <button class="md-btn md-text-btn" onclick={confirmAdd}>确定</button>
  {/snippet}
</Dialog>

<style>
  .dialog-body {
    display: flex;
    flex-direction: column;
  }
  .gap-8 {
    height: 8px;
    flex-shrink: 0;
  }
  .switch-host {
    display: inline-flex;
  }
</style>
