<script>
  /**
   * VpnSegmentPage —— 对应 vpn_segment_page.dart。
   * 仅 Android 入口（SettingsMainPage 条件展示），页面照写。
   * 注意：原版新增/编辑不 trim、不判重（isNotEmpty 即写入）。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { t } from '$lib/i18n/index.svelte.js';

  const vpnList = $derived(sm.vpnState.customVpn);

  /* 对话框状态 */
  let addOpen = $state(false);
  let addInput = $state('');
  let editOpen = $state(false);
  let editInput = $state('');
  let editIndex = $state(-1);
  let deleteOpen = $state(false);
  let deleteIndex = $state(-1);
  let deleteItem = $state('');

  function openAddDialog() {
    addInput = '';
    addOpen = true;
  }

  async function confirmAdd() {
    const value = addInput;
    addOpen = false;
    if (value !== '') {
      sm.vpnState.addCustomVpn(value);
      await sm.updateCustomVpn(sm.vpnState.customVpn);
    }
  }

  function openEditDialog(index, item) {
    editIndex = index;
    editInput = item;
    editOpen = true;
  }

  async function confirmEdit() {
    const value = editInput;
    editOpen = false;
    if (value !== '') {
      const list = [...sm.vpnState.customVpn];
      list[editIndex] = value;
      await sm.updateCustomVpn(list);
    }
  }

  function openDeleteDialog(index, item) {
    deleteIndex = index;
    deleteItem = item;
    deleteOpen = true;
  }

  async function confirmDelete() {
    deleteOpen = false;
    sm.vpnState.removeCustomVpn(deleteIndex);
    await sm.updateCustomVpn(sm.vpnState.customVpn);
  }
</script>

<SubPage title={t('custom_vpn_segment')}>
  {#snippet actions()}
    <button class="md-icon-btn" onclick={openAddDialog}>
      <Icon name="add" size={24} color="var(--md-on-surface)" />
    </button>
  {/snippet}

  {#if vpnList.length === 0}
    <div class="empty-state">
      <Icon name="vpn_lock" size={64} color="var(--md-outline)" />
      <div class="gap-16"></div>
      <div class="empty-title">No VPN segments configured</div>
      <div class="gap-16"></div>
      <button class="md-btn md-elevated-btn" onclick={openAddDialog}>
        <Icon name="add" size={18} />
        {t('add_vpn_segment')}
      </button>
    </div>
  {:else}
    <div class="page-list">
      {#each vpnList as vpn, index (index)}
        <div class="md-card item-card">
          <div class="md-list-tile no-cursor">
            <span class="tile-leading"><Icon name="network_wifi" size={24} /></span>
            <div class="tile-content">
              <div class="tile-title">{vpn}</div>
            </div>
            <span class="tile-trailing">
              <button
                class="md-icon-btn"
                data-tip={t('edit')}
                onclick={() => openEditDialog(index, vpn)}
              >
                <Icon name="edit" size={20} />
              </button>
              <span class="trailing-gap"></span>
              <button
                class="md-icon-btn"
                data-tip={t('delete')}
                onclick={() => openDeleteDialog(index, vpn)}
              >
                <Icon name="delete" size={20} />
              </button>
            </span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</SubPage>

<!-- 添加VPN网段 -->
<Dialog bind:open={addOpen} title={t('add_vpn_segment')}>
  <TextField
    bind:value={addInput}
    label={t('vpn_segment_format_example')}
    hint={t('vpn_segment_input_hint')}
  />
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (addOpen = false)}>{t('cancel')}</button>
    <button class="md-btn md-text-btn" onclick={confirmAdd}>{t('add')}</button>
  {/snippet}
</Dialog>

<!-- 编辑VPN网段 -->
<Dialog bind:open={editOpen} title={t('edit_vpn_segment')}>
  <TextField bind:value={editInput} label={t('vpn_segment_format_example')} />
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (editOpen = false)}>{t('cancel')}</button>
    <button class="md-btn md-text-btn" onclick={confirmEdit}>{t('save')}</button>
  {/snippet}
</Dialog>

<!-- 确认删除 -->
<Dialog bind:open={deleteOpen} title={t('confirm_delete')}>
  <div>{t('confirm_delete_vpn_segment', { vpn: deleteItem })}</div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (deleteOpen = false)}>{t('cancel')}</button>
    <button class="md-btn md-text-btn delete-btn" onclick={confirmDelete}>{t('delete')}</button>
  {/snippet}
</Dialog>

<style>
  .page-list {
    padding: 16px;
  }
  .item-card {
    margin-bottom: 8px;
  }
  .empty-state {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 16px;
    box-sizing: border-box;
  }
  .empty-title {
    font-size: 16px;
    line-height: 24px;
    font-weight: 500;
    color: var(--md-outline);
  }
  .gap-16 {
    height: 16px;
  }
  .trailing-gap {
    width: 4px;
  }
  /* TextButton red 前景 */
  .delete-btn {
    color: var(--flutter-red);
  }
</style>
