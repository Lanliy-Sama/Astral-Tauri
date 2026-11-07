<script>
  /**
   * 端口白名单子页 —— 对应 lib/features/settings/pages/network/port_whitelist_page.dart。
   * 三张设置卡（说明 / TCP / UDP）+ 编辑对话框 + 清空确认对话框。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';

  const tcpWhitelist = $derived(sm.networkConfigState.tcpWhitelist);
  const udpWhitelist = $derived(sm.networkConfigState.udpWhitelist);

  /* 编辑对话框 */
  let editOpen = $state(false);
  let editType = $state('TCP');
  let editValue = $state('');

  /* 清空确认 */
  let clearOpen = $state(false);
  let clearType = $state('TCP');

  function openEdit(type) {
    editType = type;
    editValue = type === 'TCP' ? sm.networkConfigState.tcpWhitelist : sm.networkConfigState.udpWhitelist;
    editOpen = true;
  }

  function saveEdit() {
    const value = editValue.trim();
    if (editType === 'TCP') sm.updateTcpWhitelist(value);
    else sm.updateUdpWhitelist(value);
    editOpen = false;
  }

  function openClear(type) {
    clearType = type;
    clearOpen = true;
  }

  function confirmClear() {
    if (clearType === 'TCP') sm.updateTcpWhitelist('');
    else sm.updateUdpWhitelist('');
    clearOpen = false;
  }
</script>

<SubPage title="端口白名单">
  <div class="pw-body">
    <!-- 白名单说明 -->
    <div class="md-card pw-card">
      <div class="md-list-tile no-cursor pw-tile">
        <span class="tile-leading">
          <!-- Icons.info_outline：经基名字形渲染描边 info -->
          <span class="ms" style="font-size:24px; color:var(--md-on-surface-variant)">info</span>
        </span>
        <span class="tile-content">
          <span class="tile-title">白名单说明</span>
          <span class="tile-subtitle pw-desc">{'配置允许通过的TCP/UDP端口。\n支持单个端口(80)和范围(8000-9000)。\n多个端口用逗号分隔，如: 80,443,8000-9000'}</span>
        </span>
      </div>
    </div>

    <div class="gap16"></div>

    <!-- TCP 端口白名单 -->
    <div class="md-card pw-card">
      <div class="md-list-tile no-cursor pw-tile">
        <span class="tile-leading">
          <Icon name="storage" size={24} color="var(--md-on-surface-variant)" />
        </span>
        <span class="tile-content">
          <span class="tile-title">TCP 端口白名单</span>
          <span
            class="tile-subtitle"
            style="color:{tcpWhitelist ? 'var(--md-primary)' : 'var(--md-on-surface)'}"
          >
            {tcpWhitelist || '未设置'}
          </span>
        </span>
        <span class="tile-trailing">
          <button class="md-icon-btn" data-tip="编辑" onclick={() => openEdit('TCP')}>
            <Icon name="edit" size={24} />
          </button>
          {#if tcpWhitelist}
            <button class="md-icon-btn" data-tip="清空" onclick={() => openClear('TCP')}>
              <Icon name="close" size={24} />
            </button>
          {/if}
        </span>
      </div>
    </div>

    <div class="gap16"></div>

    <!-- UDP 端口白名单 -->
    <div class="md-card pw-card">
      <div class="md-list-tile no-cursor pw-tile">
        <span class="tile-leading">
          <Icon name="wifi" size={24} color="var(--md-on-surface-variant)" />
        </span>
        <span class="tile-content">
          <span class="tile-title">UDP 端口白名单</span>
          <span
            class="tile-subtitle"
            style="color:{udpWhitelist ? 'var(--md-primary)' : 'var(--md-on-surface)'}"
          >
            {udpWhitelist || '未设置'}
          </span>
        </span>
        <span class="tile-trailing">
          <button class="md-icon-btn" data-tip="编辑" onclick={() => openEdit('UDP')}>
            <Icon name="edit" size={24} />
          </button>
          {#if udpWhitelist}
            <button class="md-icon-btn" data-tip="清空" onclick={() => openClear('UDP')}>
              <Icon name="close" size={24} />
            </button>
          {/if}
        </span>
      </div>
    </div>
  </div>
</SubPage>

<!-- 编辑端口白名单 -->
<Dialog bind:open={editOpen} title={'编辑 ' + editType + ' 端口白名单'}>
  <div class="fmt-col">
    <div class="fmt-title">格式说明：</div>
    <div class="gap8"></div>
    <div>• 单个端口: 80</div>
    <div>• 端口范围: 8000-9000</div>
    <div>• 多个端口: 80,443,8000-9000</div>
    <div class="gap16"></div>
    <TextField
      bind:value={editValue}
      label={editType + ' 端口'}
      hint="例如: 80,443,8000-9000"
      helperText="留空表示不限制"
      multiline={true}
      rows={3}
      autofocus={true}
      style="width:100%"
    />
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (editOpen = false)}>取消</button>
    <button class="md-btn md-text-btn" onclick={saveEdit}>保存</button>
  {/snippet}
</Dialog>

<!-- 确认清空 -->
<Dialog bind:open={clearOpen} title="确认清空">
  <div class="clear-text">{'确定要清空 ' + clearType + ' 端口白名单吗？\n清空后将不限制端口访问。'}</div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (clearOpen = false)}>取消</button>
    <button class="md-btn md-text-btn" onclick={confirmClear}>清空</button>
  {/snippet}
</Dialog>

<style>
  /* ListView padding 16 */
  .pw-body {
    padding: 16px;
  }
  .gap8 {
    height: 8px;
  }
  .gap16 {
    height: 16px;
  }
  /* ListTile 无 onTap → 无水波/hover */
  .pw-card .md-list-tile:hover,
  .pw-card .md-list-tile:active {
    background: transparent;
  }
  /* 三行说明（\n 换行） */
  .pw-desc {
    white-space: pre-line;
  }
  /* 编辑对话框：Column(crossAxisAlignment.start) */
  .fmt-col {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .fmt-title {
    font-weight: 700;
  }
  .clear-text {
    white-space: pre-line;
  }
</style>
