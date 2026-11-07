<script>
  /**
   * ListenListPage —— 对应 listen_list_page.dart。
   * 监听列表增删改 + 确认删除（数据 playerState.listenList）。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { t } from '$lib/i18n/I18n.svelte';

  const listenList = $derived(sm.playerState.listenList);

  /* 对话框状态 */
  let addOpen = $state(false);
  let addInput = $state('');
  let editOpen = $state(false);
  let editInput = $state('');
  let editIndex = $state(-1);
  let editItem = $state('');
  let deleteOpen = $state(false);
  let deleteIndex = $state(-1);
  let deleteItem = $state('');

  function openAddDialog() {
    addInput = '';
    addOpen = true;
  }

  async function confirmAdd() {
    const value = addInput.trim();
    addOpen = false;
    if (value !== '') {
      await sm.addListen(value);
    }
  }

  function openEditDialog(index, item) {
    editIndex = index;
    editItem = item;
    editInput = item;
    editOpen = true;
  }

  async function confirmEdit() {
    const value = editInput.trim();
    editOpen = false;
    if (value !== '' && value !== editItem) {
      await sm.updateListen(editIndex, value);
    }
  }

  function openDeleteDialog(index, item) {
    deleteIndex = index;
    deleteItem = item;
    deleteOpen = true;
  }

  async function confirmDelete() {
    deleteOpen = false;
    await sm.deleteListen(deleteIndex);
  }
</script>

<SubPage title={t('listen_list')}>
  {#snippet actions()}
    <button class="md-icon-btn" onclick={openAddDialog}>
      <Icon name="add" size={24} color="var(--md-on-surface)" />
    </button>
  {/snippet}

  {#if listenList.length === 0}
    <div class="empty-state">
      <Icon name="list_alt" size={64} color="var(--md-outline)" />
      <div class="gap-16"></div>
      <div class="empty-title">暂无监听项</div>
      <div class="gap-16"></div>
      <button class="md-btn md-elevated-btn" onclick={openAddDialog}>
        <Icon name="add" size={18} />
        {t('add_listen_item')}
      </button>
    </div>
  {:else}
    <div class="page-list">
      {#each listenList as item, index (index)}
        <div class="md-card item-card">
          <div class="md-list-tile no-cursor">
            <span class="tile-leading"><Icon name="dns" size={24} /></span>
            <div class="tile-content">
              <div class="tile-title">{item}</div>
            </div>
            <span class="tile-trailing">
              <button
                class="md-icon-btn"
                data-tip={t('edit')}
                onclick={() => openEditDialog(index, item)}
              >
                <Icon name="edit" size={20} />
              </button>
              <span class="trailing-gap"></span>
              <button
                class="md-icon-btn"
                data-tip={t('delete')}
                onclick={() => openDeleteDialog(index, item)}
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

<!-- 新增监听项 -->
<Dialog bind:open={addOpen} title={t('add_listen_item')}>
  <TextField bind:value={addInput} autofocus label={t('listen_item')} hint="localhost:8080" />
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (addOpen = false)}>{t('cancel')}</button>
    <button class="md-btn md-text-btn" onclick={confirmAdd}>{t('add')}</button>
  {/snippet}
</Dialog>

<!-- 编辑监听项 -->
<Dialog bind:open={editOpen} title={t('edit_listen_item')}>
  <TextField bind:value={editInput} autofocus label={t('listen_item')} />
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (editOpen = false)}>{t('cancel')}</button>
    <button class="md-btn md-text-btn" onclick={confirmEdit}>{t('save')}</button>
  {/snippet}
</Dialog>

<!-- 确认删除 -->
<Dialog bind:open={deleteOpen} title={t('confirm_delete')}>
  <div>{t('confirm_delete_listen_item', { item: deleteItem })}</div>
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
