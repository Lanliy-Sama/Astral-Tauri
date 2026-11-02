<script>
  /**
   * 添加 / 编辑服务器对话框 —— 对应 lib/features/servers/dialogs/server_dialog.dart。
   * server 为 null 时是添加模式（'添加服务器'/'添加'，默认 tcp/udp 开），否则编辑模式（'编辑服务器'/'保存'）。
   * 被屏蔽地址（BlockedServers）编辑时输入框禁用并显示 helper。
   */
  import Dialog from '$lib/components/common/Dialog.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { isBlockedServer } from '$lib/utils/network.js';

  let { open = $bindable(false), server = null } = $props();

  /* 11 个协议开关（Wrap spacing 16 / runSpacing 8） */
  const PROTOCOLS = [
    { key: 'tcp', label: 'TCP' },
    { key: 'faketcp', label: 'FAKETCP' },
    { key: 'udp', label: 'UDP' },
    { key: 'ws', label: 'WS' },
    { key: 'wss', label: 'WSS' },
    { key: 'quic', label: 'QUIC' },
    { key: 'wg', label: 'WG' },
    { key: 'txt', label: 'TXT' },
    { key: 'srv', label: 'SRV' },
    { key: 'http', label: 'HTTP' },
    { key: 'https', label: 'HTTPS' },
  ];

  let name = $state('');
  let url = $state('');
  let nameError = $state('');
  let urlError = $state('');
  let proto = $state({
    tcp: true,
    faketcp: false,
    udp: true,
    ws: false,
    wss: false,
    quic: false,
    wg: false,
    txt: false,
    srv: false,
    http: false,
    https: false,
  });

  const isEdit = $derived(server != null);
  const title = $derived(isEdit ? '编辑服务器' : '添加服务器');
  const confirmText = $derived(isEdit ? '保存' : '添加');
  const urlLocked = $derived(isEdit && isBlockedServer(server.url));

  /* 打开时填充：编辑模式回填现有数据；新增模式默认值（tcp/udp 开，其余关） */
  $effect(() => {
    if (open) {
      name = server?.name ?? '';
      url = server?.url ?? '';
      proto = {
        tcp: server ? !!server.tcp : true,
        faketcp: server ? !!server.faketcp : false,
        udp: server ? !!server.udp : true,
        ws: server ? !!server.ws : false,
        wss: server ? !!server.wss : false,
        quic: server ? !!server.quic : false,
        wg: server ? !!server.wg : false,
        txt: server ? !!server.txt : false,
        srv: server ? !!server.srv : false,
        http: server ? !!server.http : false,
        https: server ? !!server.https : false,
      };
      nameError = '';
      urlError = '';
    }
  });

  function save() {
    /* Form.validate()：非空校验（与 Dart validator 一致，空串报错） */
    nameError = name.length === 0 ? '请输入服务器名称' : '';
    urlError = url.length === 0 ? '请输入服务器地址' : '';
    if (nameError || urlError) return;

    const mod = {
      id: server?.id ?? null,
      enable: server?.enable ?? false,
      name: name,
      url: url,
      sortOrder: server?.sortOrder ?? 0,
      ...proto,
    };
    if (server == null) {
      sm.addServer(mod);
    } else {
      sm.updateServer(mod);
    }
    open = false;
  }
</script>

<div class="sd-host">
  <Dialog bind:open {title}>
    <div class="form-col">
      <TextField
        bind:value={name}
        label="服务器名称"
        hint="输入服务器名称"
        errorText={nameError}
      />
      <div class="gap16"></div>
      <TextField
        bind:value={url}
        label="服务器地址"
        hint="输入服务器地址"
        errorText={urlError}
        helperText={urlLocked ? '此服务器地址不可修改' : ''}
        disabled={urlLocked}
      />
      <div class="gap24"></div>
      <div class="proto-title">支持的协议:</div>
      <div class="gap8"></div>
      <div class="proto-wrap">
        {#each PROTOCOLS as p (p.key)}
          <div class="proto-item">
            <Switch value={proto[p.key]} onChange={(v) => (proto[p.key] = v)} />
            <span class="proto-label">{p.label}</span>
          </div>
        {/each}
      </div>
    </div>
    {#snippet actions()}
      <button class="md-btn md-text-btn" onclick={() => (open = false)}>取消</button>
      <button class="md-btn md-filled-btn" onclick={save}>{confirmText}</button>
    {/snippet}
  </Dialog>
</div>

<style>
  /* AlertDialog 无宽度上限：Wrap 内容单行可达 ~1100px，对齐 Flutter insetPadding(40) 行为 */
  .sd-host :global(.md-dialog) {
    max-width: calc(100vw - 80px);
  }
  .form-col {
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }
  .gap8 {
    height: 8px;
  }
  .gap16 {
    height: 16px;
  }
  .gap24 {
    height: 24px;
  }
  .proto-title {
    font-weight: 700;
  }
  .proto-wrap {
    display: flex;
    flex-wrap: wrap;
    column-gap: 16px;
    row-gap: 8px;
  }
  .proto-item {
    display: flex;
    align-items: center;
  }
  .proto-label {
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface);
  }
</style>
