<script>
  /**
   * SoftwareSettingsPage —— 对应 software_settings_page.dart（桌面路径）。
   * Android 权限分支（install_mobile / Android 设置卡）按规格桌面忽略。
   * 液态玻璃增强：分段控件选中块为滑动玻璃胶囊，支持按住拖动选择。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import SettingsCard from './parts/SettingsCard.svelte';
  import BareDropdown from './parts/BareDropdown.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { t } from '$lib/i18n/index.svelte.js';
  import { isAndroid } from '$lib/utils/platform.js';

  const retryOptions = [1, 2, 3, 5, 10].map((n) => ({ value: n, label: `${n} 次` }));

  /* SwitchListTile 整行可点；点 Switch 本体时避免与 tile 冒泡双触发 */
  function switchTileClick(e, current, update) {
    if (e.target.closest('.md-switch')) return;
    update(!current);
  }

  /* ---- 分段控件：滑动胶囊 + 按住拖动选择 ---- */
  let segEl = null; // .md-segmented 容器
  let pillEl = null; // 胶囊
  let segEls = []; // 各段
  let pillX = $state(0); // 胶囊位移（选中段 offsetLeft）
  let pillW = $state(0); // 胶囊宽度（选中段 offsetWidth）
  let pillSquish = $state(1); // 按压收缩
  let pillLive = $state(false); // 拖动跟随中（禁用过渡）
  let specX = $state(null); // 镜面高光 X（拖拽跟随；写进模板 style 避免重写抹掉）
  let specY = $state(null); // 镜面高光 Y
  let segDrag = null; // { targetIdx, moved }
  let suppressClick = false;

  const SEG_OPTIONS = [0, 1, 2];

  /* 选中段变化 → 胶囊滑到该段（测量 offsetLeft/offsetWidth，inset 2px 与原 M3 选中段一致） */
  $effect(() => {
    const style = sm.displayState.userListStyle;
    const seg = segEls[style];
    if (seg) {
      pillX = seg.offsetLeft + 2;
      pillW = seg.offsetWidth - 4;
    }
  });

  /* 窗口尺寸变化：段宽为弹性百分比，重新测量胶囊位置 */
  $effect(() => {
    const onResize = () => {
      const seg = segEls[sm.displayState.userListStyle];
      if (seg) {
        pillX = seg.offsetLeft + 2;
        pillW = seg.offsetWidth - 4;
      }
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  function indexFromX(x) {
    let best = 0;
    let bestD = Infinity;
    for (let i = 0; i < segEls.length; i++) {
      const el = segEls[i];
      if (!el) continue;
      const c = el.offsetLeft + el.offsetWidth / 2;
      const d = Math.abs(c - x);
      if (d < bestD) {
        bestD = d;
        best = i;
      }
    }
    return best;
  }

  function onSegPointerDown(e) {
    segDrag = { moved: false };
    pillSquish = 0.93; // 按住：胶囊液态收缩（按住分段控件的选中块）
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  function onSegPointerMove(e) {
    if (!segDrag) return;
    const rect = segEl.getBoundingClientRect();
    const idx = indexFromX(e.clientX - rect.left);
    // 镜面高光跟随（capture 下 MainScreen 委派不命中胶囊，这里直写）
    specX = e.clientX - rect.left;
    specY = e.clientY - rect.top;
    if (idx !== sm.displayState.userListStyle) {
      segDrag.moved = true;
      pillLive = true; // 拖动跟随：实时滑到指针下分段
      const seg = segEls[idx];
      if (seg) {
        pillX = seg.offsetLeft + 2;
        pillW = seg.offsetWidth - 4;
      }
    }
  }

  function onSegPointerUp() {
    if (!segDrag) return;
    const moved = segDrag.moved;
    segDrag = null;
    pillSquish = 1;
    pillLive = false;
    specX = null; // 高光回落默认位置
    specY = null;
    if (moved) {
      suppressClick = true;
      sm.setUserListStyle(nearestSegIndex()); // 松手选中胶囊停留的分段
    }
  }

  function nearestSegIndex() {
    // 胶囊当前停留的分段 = pillX 对应段（含 2px inset，四舍五入到最近段中心）
    let best = 0;
    let bestD = Infinity;
    for (let i = 0; i < segEls.length; i++) {
      const el = segEls[i];
      if (!el) continue;
      const d = Math.abs(el.offsetLeft + el.offsetWidth / 2 - (pillX + pillW / 2));
      if (d < bestD) {
        bestD = d;
        best = i;
      }
    }
    return best;
  }

  function onSegClick(idx) {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    sm.setUserListStyle(idx);
  }
</script>

<SubPage title={t('software_settings')}>
  <div class="page-list">
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="settings" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('software_settings')}</div>
          <div class="tile-subtitle">{t('software_behavior_desc')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      {#if !isAndroid()}
        <div
          class="md-switch-list-tile"
          onclick={(e) => switchTileClick(e, sm.windowState.closeMinimize, sm.updateCloseMinimize)}
        >
          <div class="tile-content">
            <div class="tile-title">{t('minimize')}</div>
            <div class="tile-subtitle">{t('minimize_desc')}</div>
          </div>
          <Switch
            value={sm.windowState.closeMinimize}
            onChange={(v) => sm.updateCloseMinimize(v)}
          />
        </div>
      {/if}

      <div class="md-list-tile two-line no-cursor">
        <div class="tile-content">
          <div class="tile-title">{t('player_list_card')}</div>
          <div class="tile-subtitle">{t('player_list_card_desc')}</div>
        </div>
      </div>
      <!-- Padding(16,0,16,12) + SegmentedButton（滑动胶囊选中块：液态玻璃下为玻璃透镜） -->
      <div class="segment-wrap">
        <div class="md-segmented" bind:this={segEl}>
          <div
            class="segment-pill lg-glass lg-spec"
            class:live={pillLive}
            bind:this={pillEl}
            style="width:{pillW}px; transform: translateX({pillX}px) scaleX({pillSquish}){specX != null ? `; --lg-x: ${specX}px; --lg-y: ${specY}px` : ''}"
          ></div>
          {#each SEG_OPTIONS as idx (idx)}
            <button
              class="segment {sm.displayState.userListStyle === idx ? 'selected' : ''}"
              bind:this={segEls[idx]}
              onclick={() => onSegClick(idx)}
              onpointerdown={onSegPointerDown}
              onpointermove={onSegPointerMove}
              onpointerup={onSegPointerUp}
              onpointercancel={onSegPointerUp}
            >{['简约', '详细', '列表'][idx]}</button>
          {/each}
        </div>
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.appSettingsState.reduceAnimationUpdates, sm.updateReduceAnimationUpdates)}
      >
        <div class="tile-content">
          <div class="tile-title">减少动画更新</div>
          <div class="tile-subtitle">降低拓扑图与连线动画刷新频率，减少 GPU 占用</div>
        </div>
        <Switch
          value={sm.appSettingsState.reduceAnimationUpdates}
          onChange={(v) => sm.updateReduceAnimationUpdates(v)}
        />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, sm.appSettingsState.liquidGlass, sm.updateLiquidGlass)}
      >
        <div class="tile-content">
          <div class="tile-title">液态玻璃效果</div>
          <div class="tile-subtitle">开关、侧边栏选中框、选项卡、分段控件、卡片与按钮使用液态玻璃材质（边缘折射 · 亮边 · 色散 · 按压流光），支持按住拖动选择</div>
        </div>
        <Switch
          value={sm.appSettingsState.liquidGlass}
          onChange={(v) => sm.updateLiquidGlass(v)}
        />
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="sync" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">连接设置</div>
          <div class="tile-subtitle">配置连接重试行为</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.appSettingsState.autoRetryOnFailure, sm.updateAutoRetryOnFailure)}
      >
        <div class="tile-content">
          <div class="tile-title">连接失败自动重试</div>
          <div class="tile-subtitle">连接失败时自动重新尝试连接</div>
        </div>
        <Switch
          value={sm.appSettingsState.autoRetryOnFailure}
          onChange={(v) => sm.updateAutoRetryOnFailure(v)}
        />
      </div>

      {#if sm.appSettingsState.autoRetryOnFailure}
        <div class="md-list-tile two-line no-cursor">
          <div class="tile-content">
            <div class="tile-title">最大重试次数</div>
            <div class="tile-subtitle">当前设置为 {sm.appSettingsState.maxRetryCount} 次</div>
          </div>
          <!-- SizedBox(width:100) + DropdownButton(isExpanded) -->
          <span class="tile-trailing retry-dd">
            <BareDropdown
              value={sm.appSettingsState.maxRetryCount}
              options={retryOptions}
              underline={true}
              width={100}
              onChange={(v) => sm.updateMaxRetryCount(v)}
            />
          </span>
        </div>
      {/if}
    </SettingsCard>
  </div>
</SubPage>

<style>
  .page-list {
    padding: 16px;
  }
  .gap-16 {
    height: 16px;
  }
  .segment-wrap {
    padding: 0 16px 12px;
  }
  /* SegmentedButton 段等宽铺满 */
  .segment-wrap .md-segmented {
    display: flex;
    width: 100%;
    position: relative; /* 滑动胶囊的定位父级 */
  }
  .segment-wrap .segment {
    flex: 1 1 0;
    z-index: 1; /* 段内容在胶囊之上 */
    touch-action: none; /* 允许按住横向拖动选择（触屏） */
  }
  /* 滑动胶囊取代静态 ::after（两种模式统一为滑动块） */
  .segment-wrap .md-segmented .segment.selected::after {
    content: none;
  }
  /* 胶囊：非液态玻璃 = 原 M3 选中段样式（secondary-container 胶囊 inset 2px） */
  .segment-pill {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 0;
    border-radius: 999px;
    background: var(--md-secondary-container);
    pointer-events: none;
    will-change: transform;
  }
  /* 液态玻璃：滑动带过冲回弹（squash & stretch） */
  :global(html.liquid-glass) .segment-pill {
    background: transparent; /* 材质由 liquid-glass.css 双层伪元素承载 */
    border-color: transparent;
    box-shadow: none;
    transition: transform 340ms cubic-bezier(0.3, 1.4, 0.45, 1);
  }
  :global(html.liquid-glass) .segment-pill.live {
    transition: none; /* 拖动跟随中由 JS 直控 */
  }
</style>
