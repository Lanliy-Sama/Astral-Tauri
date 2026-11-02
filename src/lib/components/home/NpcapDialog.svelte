<script>
  /**
   * Npcap 驱动提示对话框 —— 对照 lib/features/home/widgets/connect_npcap_guard.dart
   * 中的 showNpcapRequiredDialog。返回值经 onresult 回调（true=查看教程 / false=取消）。
   */
  import Dialog from '../common/Dialog.svelte';

  let { open = $bindable(false), onresult = () => {} } = $props();

  function close(v) {
    open = false;
    onresult(v);
  }
</script>

<Dialog bind:open title="需要 Npcap 驱动" onDismiss={() => onresult(false)}>
  <div class="npcap-text">
    检测到当前连接包含 FakeTCP 服务器。
    Windows 需要先安装 Npcap 驱动后才能使用 FakeTCP。

    是否前往 astral.fan 查看安装教程？
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => close(false)}>取消</button>
    <button class="md-btn md-filled-btn" onclick={() => close(true)}>查看教程</button>
  {/snippet}
</Dialog>

<style>
  .npcap-text {
    white-space: pre-line;
  }
</style>
