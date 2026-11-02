<script>
  /**
   * 编辑房间对话框 —— 翻译自 features/rooms/dialogs/edit_room_dialog.dart。
   * customParam 非空 → 「无法编辑房间」拦截提示态；正常态编辑名称/房间号/密码。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';

  let { open = $bindable(false), room = null } = $props();

  let name = $state('');
  let roomName = $state('');
  let roomPassword = $state('');

  /* 含自定义参数（携带服务器分享而来）的房间不可编辑 */
  const blocked = $derived(!!room && (room.customParam || '') !== '');

  $effect(() => {
    if (open && room) {
      name = room.name || '';
      roomName = room.roomName || '';
      roomPassword = room.password || '';
    }
  });

  async function confirmEdit() {
    if (!room) return;
    await sm.updateRoom({
      ...room,
      name,
      /* 加密房间的房间号/密码不可改（原版不渲染这两个输入框） */
      roomName: room.encrypted ? room.roomName : roomName,
      password: room.encrypted ? room.password : roomPassword,
    });
    open = false;
  }
</script>

{#if blocked}
  <!-- 拦截提示态 -->
  <Dialog bind:open={open} title="无法编辑房间">
    <div class="blocked-body">
      <Icon name="lock" size={48} color="var(--md-primary)" />
      <div class="blocked-title">此房间含有自定义参数</div>
      <div class="blocked-text">
        含有自定义参数的房间不可编辑，只能删除。如需修改，请删除此房间并重新创建或导入。
      </div>
    </div>
    {#snippet actions()}
      <button class="md-btn md-text-btn" onclick={() => (open = false)}>我知道了</button>
    {/snippet}
  </Dialog>
{:else}
  <Dialog bind:open={open} title="编辑房间">
    <div class="dialog-body">
      <TextField bind:value={name} label="房间名称" />
      <div class="gap-8"></div>
      <!-- 房间类型（只读 ListTile） -->
      <div class="md-list-tile two-line no-cursor type-tile">
        <div class="tile-content">
          <div class="tile-title">房间类型</div>
          <div class="tile-subtitle">{room?.encrypted ? '加密房间' : '普通房间'}</div>
        </div>
      </div>
      {#if room && !room.encrypted}
        <TextField bind:value={roomName} label="房间号" />
        <div class="gap-8"></div>
        <TextField bind:value={roomPassword} label="房间密码" />
      {/if}
    </div>
    {#snippet actions()}
      <button class="md-btn md-text-btn" onclick={() => (open = false)}>取消</button>
      <button class="md-btn md-text-btn" onclick={confirmEdit}>确定</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .dialog-body {
    display: flex;
    flex-direction: column;
  }
  .gap-8 {
    height: 8px;
    flex-shrink: 0;
  }

  /* 拦截提示态：Column(center) */
  .blocked-body {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }
  .blocked-title {
    margin-top: 16px;
    font-size: 14px;
    line-height: 20px;
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .blocked-text {
    margin-top: 8px;
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface);
  }
</style>
