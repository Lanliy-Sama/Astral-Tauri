<script>
  /**
   * buildSettingsCard 等价（base_settings_page.dart）：
   * 可选 header（Padding l16,t8,r16,b8 + titleSmall primary bold）+ Card(Column)。
   * 子项之间需要分隔线时在 children 里放 <div class="md-divider"></div>。
   */
  let { header = null, children } = $props();
</script>

<div class="settings-card">
  {#if header}
    <div class="card-header">{header}</div>
  {/if}
  <div class="md-card">
    <div class="card-column">
      {@render children?.()}
    </div>
  </div>
</div>

<style>
  .settings-card {
    display: flex;
    flex-direction: column;
    width: 100%;
    align-items: stretch;
  }
  /* titleSmall + bold + primary */
  .card-header {
    padding: 8px 16px;
    font-size: 14px;
    line-height: 20px;
    font-weight: 700;
    letter-spacing: 0.1px;
    color: var(--md-primary);
  }
  .card-column {
    display: flex;
    flex-direction: column;
  }
</style>
