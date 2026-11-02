<script>
  /**
   * MagicWallControlPanel —— 对应 magic_wall_control_panel.dart。
   * 引擎开关卡：标题(18,bold) + 状态文字(14, green/grey) + Switch（非 Windows 禁用）；
   * 非 Windows 追加平台提示（12, orange）。
   */
  import Switch from '$lib/components/common/Switch.svelte';
  import { isWindows } from '$lib/utils/platform.js';

  let { isRunning = false, onToggle = () => {} } = $props();
</script>

<div class="md-card mw-panel">
  <div class="mw-panel-inner">
    <div class="mw-row">
      <div class="mw-info">
        <div class="mw-title">魔法墙引擎</div>
        <div class="mw-status" class:running={isRunning}>
          {isRunning ? '运行中' : '已停止'}
        </div>
      </div>
      <!-- Platform.isWindows ? (value) => onToggle() : null -->
      <Switch
        value={isRunning}
        disabled={!isWindows()}
        onChange={() => onToggle()}
      />
    </div>
    {#if !isWindows()}
      <div class="mw-warning">⚠️ 魔法墙仅支持 Windows 平台</div>
    {/if}
  </div>
</div>

<style>
  /* Card(margin: EdgeInsets.all(16)) */
  .mw-panel {
    margin: 16px;
  }
  /* Padding(16) Column(crossAxisAlignment.start) */
  .mw-panel-inner {
    padding: 16px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .mw-row {
    display: flex;
    align-items: center;
    width: 100%;
  }
  /* Expanded Column(start) */
  .mw-info {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .mw-title {
    font-size: 18px;
    line-height: 24px;
    font-weight: 700;
    color: var(--md-on-surface);
  }
  /* SizedBox(height: 4) */
  .mw-status {
    margin-top: 4px;
    font-size: 14px;
    line-height: 20px;
    color: var(--flutter-grey); /* Colors.grey */
  }
  .mw-status.running {
    color: var(--flutter-green); /* Colors.green */
  }
  /* SizedBox(height: 8) + Text(12, Colors.orange) */
  .mw-warning {
    margin-top: 8px;
    font-size: 12px;
    line-height: 16px;
    color: var(--flutter-orange);
  }
</style>
