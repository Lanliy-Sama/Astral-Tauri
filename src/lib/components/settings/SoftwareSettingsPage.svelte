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
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { t } from '$lib/i18n/I18n.svelte';
  import { isAndroid, isTauri } from '$lib/utils/Platform.svelte';
  import { snackError, snackSuccess, snackInfo } from '$lib/components/common/SnackbarStore.svelte';

  const retryOptions = [1, 2, 3, 5, 10].map((n) => ({ value: n, label: `${n} 次` }));

  /* SwitchListTile 整行可点；点 Switch 本体时避免与 tile 冒泡双触发 */
  function switchTileClick(e, current, update) {
    if (e.target.closest('.md-switch')) return;
    update(!current);
  }

  /* ============================================================
   * 分段控件：滑动胶囊 + 按住拖动选择（液态玻璃）
   * ------------------------------------------------------------
   * v2.6 重写为与侧边栏「拖一拖滑块」同源的连续交互：
   *  - 按住（玻璃开）：胶囊液态收缩，进入热透镜（强折射+液态流动）；
   *  - 拖动：胶囊中心连续跟随指针（不再离散跳段），速度映射为横向
   *    液态拉伸/纵向压缩（squash & stretch），指针下的段即时高亮；
   *  - 松手：带初速度的欠阻尼弹簧吸附最近段（轻微过冲回弹）；
   *  - 点击：弹簧滑动到目标段（rAF 驱动，两种模式一致）；
   *  - 玻璃关：仅点击切换（不建立拖动会话/形变）。
   * ============================================================ */
  let segEl = null; // .md-segmented 容器
  let segEls = []; // 各段
  let pillX = $state(0); // 胶囊位移（选中段 offsetLeft + 2）
  let pillW = $state(0); // 胶囊宽度（选中段 offsetWidth - 4）
  let pillScaleX = $state(1); // 液态形变：横向（拉伸/收缩）
  let pillScaleY = $state(1); // 液态形变：纵向（体积补偿压缩）
  let specX = $state(null); // 镜面高光 X（拖拽跟随；写进模板 style 避免重写抹掉）
  let specY = $state(null); // 镜面高光 Y
  let hoveredSeg = $state(null); // 拖动中指针下的段（即时高亮）
  let segDrag = null; // { v, lastX, lastT, moved }
  let suppressClick = false;
  let springRaf = 0;
  let springTarget = null; // 当前弹簧目标 x（去重，拖拽提交时避免二次重启）
  let currentX = null; // 胶囊当前 x（不进响应式，rAF 内直写）
  let deformRaf = 0; // 形变松弛 rAF
  let deformTargetX = 1; // 形变目标（按压收缩 / 拉伸 / 复原）
  let deformTargetY = 1;

  const SEG_OPTIONS = [0, 1, 2];

  function segGeom(idx) {
    const el = segEls[idx];
    return el ? { x: el.offsetLeft + 2, w: el.offsetWidth - 4, c: el.offsetLeft + el.offsetWidth / 2 } : { x: 0, w: 0, c: 0 };
  }

  /* 弹簧：欠阻尼积分器（点击切换 / 松手回弹共用），v0 供松手时接续拖拽速度 */
  function springTo(target, v0 = 0) {
    cancelAnimationFrame(springRaf);
    springTarget = target;
    let x = currentX == null ? target : currentX;
    let v = v0;
    const stiffness = 420;
    const damping = 33;
    let lastT = performance.now();
    const step = (t) => {
      const dt = Math.min((t - lastT) / 1000, 0.032);
      lastT = t;
      const a = -stiffness * (x - target) - damping * v;
      v += a * dt;
      x += v * dt;
      currentX = x;
      // 液态拉伸：回弹速度 → 横向拉伸（cap 30%）
      const stretch = Math.min(Math.abs(v) / 1500, 0.3);
      pillX = x;
      pillScaleX = 1 + stretch;
      pillScaleY = 1 / (1 + stretch * 0.45);
      if (Math.abs(x - target) < 0.12 && Math.abs(v) < 2) {
        currentX = target;
        pillX = target;
        pillScaleX = 1;
        pillScaleY = 1;
        springTarget = null;
        return;
      }
      springRaf = requestAnimationFrame(step);
    };
    springRaf = requestAnimationFrame(step);
  }

  /* 形变松弛：目标形变 → 平滑逼近（按压收缩进入 / 松手复原都走这里） */
  function ensureDeformLoop() {
    if (deformRaf) return;
    let lastT = performance.now();
    const step = (t) => {
      const dt = Math.min((t - lastT) / 1000, 0.032);
      lastT = t;
      const k = 1 - Math.exp(-dt * 18);
      pillScaleX += (deformTargetX - pillScaleX) * k;
      pillScaleY += (deformTargetY - pillScaleY) * k;
      if (
        Math.abs(pillScaleX - deformTargetX) < 0.002 &&
        Math.abs(pillScaleY - deformTargetY) < 0.002
      ) {
        pillScaleX = deformTargetX;
        pillScaleY = deformTargetY;
        deformRaf = 0;
        return;
      }
      deformRaf = requestAnimationFrame(step);
    };
    deformRaf = requestAnimationFrame(step);
  }

  /* 选中段变化 → 弹簧滑到新位置（拖拽提交时 springTarget 已对齐则跳过） */
  $effect(() => {
    const style = sm.displayState.userListStyle;
    const g = segGeom(style);
    if (currentX == null) {
      // 首次定位：直接就位，不播动画
      currentX = g.x;
      pillX = g.x;
      pillW = g.w;
      return;
    }
    pillW = g.w;
    if (springTarget === g.x) return;
    if (currentX !== g.x) springTo(g.x);
    else {
      currentX = g.x;
      pillX = g.x;
    }
  });

  /* 窗口尺寸变化：段宽为弹性百分比，重新测量胶囊位置 */
  $effect(() => {
    const onResize = () => {
      cancelAnimationFrame(springRaf);
      springTarget = null;
      const g = segGeom(sm.displayState.userListStyle);
      currentX = g.x;
      pillX = g.x;
      pillW = g.w;
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  function indexFromX(x) {
    let best = 0;
    let bestD = Infinity;
    for (let i = 0; i < segEls.length; i++) {
      const d = Math.abs(segGeom(i).c - x);
      if (d < bestD) {
        bestD = d;
        best = i;
      }
    }
    return best;
  }

  function onSegPointerDown(e) {
    /* 按住拖动胶囊（拖动选择分段）是液态玻璃专属交互：
     * 关闭液态玻璃时仅允许点击切换，不建立拖动会话/按压收缩 */
    if (!sm.appSettingsState.liquidGlass) return;
    if (e.button !== 0) return;
    segDrag = { v: 0, lastX: e.clientX, lastT: performance.now(), moved: false };
    // 按住：胶囊液态收缩（按住分段控件的选中块）
    deformTargetX = 0.94;
    deformTargetY = 0.965;
    ensureDeformLoop();
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  function onSegPointerMove(e) {
    if (!segDrag) return;
    const now = performance.now();
    const dt = Math.max(now - segDrag.lastT, 1) / 1000;
    const dx = e.clientX - segDrag.lastX;
    if (!segDrag.moved && Math.abs(e.clientX - segDrag.lastX) > 3) {
      segDrag.moved = true;
      cancelAnimationFrame(springRaf);
      springTarget = null;
      segEl.classList.add('lg-dragging'); // 热透镜 + 指针下段高亮
      currentX = pillX; // 从当前位置开始连续跟随
    }
    if (!segDrag.moved) return;
    // 速度跟踪（指数平滑）
    segDrag.v = segDrag.v * 0.72 + (dx / dt) * 0.28;
    segDrag.lastX = e.clientX;
    segDrag.lastT = now;
    // 连续跟随：胶囊中心对齐指针，锢制在首末段中心范围内
    const rect = segEl.getBoundingClientRect();
    const firstC = segGeom(0).c;
    const lastC = segGeom(SEG_OPTIONS.length - 1).c;
    const cx = Math.min(lastC, Math.max(firstC, e.clientX - rect.left));
    const x = cx - pillW / 2;
    currentX = x;
    pillX = x;
    // 液态形变：拖动速度 → 横向拉伸、纵向压缩（体积补偿）
    const stretch = Math.min(Math.abs(segDrag.v) / 1500, 0.3);
    deformTargetX = 1 + stretch;
    deformTargetY = 1 / (1 + stretch * 0.45);
    ensureDeformLoop();
    // 指针下的段即时高亮 + 镜面高光跟随（capture 下 MainScreen 委派不命中胶囊，这里直写）
    const under = indexFromX(cx);
    if (hoveredSeg !== under) hoveredSeg = under;
    specX = e.clientX - rect.left;
    specY = e.clientY - rect.top;
  }

  function onSegPointerUp() {
    if (!segDrag) return;
    const moved = segDrag.moved;
    const v = segDrag.v;
    segDrag = null;
    segEl.classList.remove('lg-dragging');
    if (hoveredSeg != null) hoveredSeg = null;
    specX = null; // 高光回落默认位置
    specY = null;
    // 形变目标复原（弹簧回弹中的速度拉伸由 springTo 接管）
    deformTargetX = 1;
    deformTargetY = 1;
    ensureDeformLoop();
    if (moved) {
      suppressClick = true;
      const idx = nearestSegIndex(); // 胶囊中心所在段
      springTo(segGeom(idx).x, v * 0.55); // 带初速度弹回
      if (sm.displayState.userListStyle !== idx) {
        sm.setUserListStyle(idx); // effect 内 springTarget 已对齐，不会重启
      }
    }
  }

  function nearestSegIndex() {
    // 胶囊当前停留的分段 = 胶囊中心最近的段中心
    let best = 0;
    let bestD = Infinity;
    const cx = currentX + pillW / 2;
    for (let i = 0; i < segEls.length; i++) {
      const d = Math.abs(segGeom(i).c - cx);
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

  /* ============================================================
   * 自定义背景：选择电脑中的任意图像文件作为 Astral 背景
   * ------------------------------------------------------------
   * v2.6 架构：彻底放弃「巨型 data URL 塞 CSS 变量」—— Chromium 对
   * CSS 自定义属性值存在 ~1.5MB 上限，超过即被静默丢弃（表象即
   * “设置后没有生效”）。新方案：
   *  - Tauri（真实应用）：原始字节按 1MB 分块 base64 经 IPC 流式写入
   *    应用数据目录（bg_write_* 命令，Rust 侧落盘），由 astralbg://
   *    自定义协议直接从磁盘服务 —— 无大小限制、内存占用恒定、
   *    GIF 自动逐帧播放、重启即恢复；
   *  - 浏览器预览：Blob URL（createObjectURL，同样无大小上限，会话级）。
   * 两种路径均先经 Image 探针验证可解码，再写入应用状态。 */
  const BG_MIME_BY_EXT = {
    jpg: 'image/jpeg', jpeg: 'image/jpeg', jfif: 'image/jpeg', pjpe: 'image/jpeg', pjp: 'image/jpeg',
    png: 'image/png', apng: 'image/apng', gif: 'image/gif', webp: 'image/webp',
    bmp: 'image/bmp', dib: 'image/bmp', svg: 'image/svg+xml',
    ico: 'image/x-icon', cur: 'image/x-icon',
    tif: 'image/tiff', tiff: 'image/tiff',
    avif: 'image/avif', heic: 'image/heic', heif: 'image/heif', heics: 'image/heic',
    jxl: 'image/jxl',
  };
  const MIME_TO_EXT = {
    'image/jpeg': 'jpg', 'image/png': 'png', 'image/apng': 'apng', 'image/gif': 'gif',
    'image/webp': 'webp', 'image/bmp': 'bmp', 'image/svg+xml': 'svg',
    'image/x-icon': 'ico', 'image/tiff': 'tiff', 'image/avif': 'avif',
    'image/heic': 'heic', 'image/heif': 'heif', 'image/heic-sequence': 'heic',
    'image/jxl': 'jxl',
  };
  const BG_ACCEPT_ATTR = `image/*,${Object.keys(BG_MIME_BY_EXT).map((e) => '.' + e).join(',')}`;
  const BG_CHUNK = 1 << 20; // 1MB 分块（IPC 单次负载安全区间）
  let bgFileEl = null; // 隐藏 file input
  let bgBusy = $state(false); // 写入中（按钮禁用）

  function resolveImageExt(file) {
    const ext = (file.name.split('.').pop() || '').toLowerCase();
    if (BG_MIME_BY_EXT[ext]) return ext;
    const mime = file.type && file.type.startsWith('image/') ? file.type : null;
    return MIME_TO_EXT[mime] || null;
  }

  function chooseBackground() {
    bgFileEl?.click();
  }

  /* Blob 切片 → base64（FileReader，避免手写分块 fromCharCode） */
  function blobToB64(blob) {
    return new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(String(r.result).split(',')[1] || '');
      r.onerror = () => reject(r.error);
      r.readAsDataURL(blob);
    });
  }

  /* Image 探针：验证 URL 可解码为图像（损坏文件/不支持的编码提前报错） */
  function probeImage(url, timeoutMs = 30000) {
    return new Promise((resolve, reject) => {
      const img = new Image();
      const timer = setTimeout(() => {
        img.src = '';
        reject(new Error('timeout'));
      }, timeoutMs);
      img.onload = () => {
        clearTimeout(timer);
        resolve();
      };
      img.onerror = () => {
        clearTimeout(timer);
        reject(new Error('decode failed'));
      };
      img.src = url;
    });
  }

  async function onBgFileChosen(e) {
    const file = e.target.files?.[0];
    e.target.value = ''; // 允许再次选择同一文件
    if (!file || bgBusy) return;
    const ext = resolveImageExt(file);
    if (!ext) {
      snackError(t('bg_invalid_type'), t('bg_invalid_type_msg'));
      return;
    }
    bgBusy = true;
    try {
      let url;
      if (isTauri()) {
        // 大文件提示（流式分块，UI 保持响应）
        if (file.size > 8 * BG_CHUNK) snackInfo(t('bg_processing'), t('bg_processing_msg'));
        const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
        await invoke('bg_write_begin', { ext });
        for (let off = 0; off < file.size; off += BG_CHUNK) {
          const b64 = await blobToB64(file.slice(off, Math.min(off + BG_CHUNK, file.size)));
          await invoke('bg_write_append', { b64 });
        }
        const info = await invoke('bg_write_end');
        // astralbg:// 逻辑路径 bg + mtime 版本号（换图即变，绕过缓存）
        url = convertFileSrc('bg', 'astralbg') + '?v=' + info.version;
      } else {
        // 浏览器预览：Blob URL（无大小上限；GIF/APNG 逐帧播放）
        url = URL.createObjectURL(file);
      }
      try {
        await probeImage(url);
      } catch {
        if (!isTauri()) URL.revokeObjectURL(url);
        snackError(t('bg_decode_failed'), t('bg_decode_failed_msg'));
        return;
      }
      await sm.updateCustomBackground(url);
      snackSuccess(t('bg_applied'), t('bg_applied_msg'));
    } catch (err) {
      snackError(t('bg_read_failed'), String(err?.message || err));
    } finally {
      bgBusy = false;
    }
  }

  async function clearBackground() {
    if (bgBusy) return;
    if (isTauri()) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('clear_custom_background');
      } catch (e) {
        console.warn('清除背景文件失败:', e);
      }
    }
    await sm.updateCustomBackground(null);
    snackInfo(t('bg_cleared'), t('bg_cleared_msg'));
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
            style="width:{pillW}px; transform: translateX({pillX}px) scaleX({pillScaleX}) scaleY({pillScaleY}){specX != null ? `; --lg-x: ${specX}px; --lg-y: ${specY}px` : ''}"
          ></div>
          {#each SEG_OPTIONS as idx (idx)}
            <button
              class="segment {sm.displayState.userListStyle === idx ? 'selected' : ''}"
              class:hovered={hoveredSeg === idx}
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

    <!-- 自定义背景：选择电脑中的任意图像文件（全部图像格式）作为应用背景 -->
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="wallpaper" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('custom_background')}</div>
          <div class="tile-subtitle">{t('custom_background_desc')}</div>
        </div>
        {#if sm.appSettingsState.customBackground}
          <span class="tile-trailing bg-preview-wrap">
            <img class="bg-preview" src={sm.appSettingsState.customBackground} alt="" />
          </span>
        {/if}
      </div>
      <div class="md-divider"></div>
      <div class="bg-actions">
        <button class="md-btn md-tonal-btn" onclick={chooseBackground} disabled={bgBusy}>
          {t('choose_image')}
        </button>
        <button
          class="md-btn md-text-btn"
          onclick={clearBackground}
          disabled={!sm.appSettingsState.customBackground || bgBusy}
        >{t('reset_background')}</button>
        {#if bgBusy}
          <span class="bg-busy-hint">{t('bg_processing')}…</span>
        {/if}
        <input
          type="file"
          accept={BG_ACCEPT_ATTR}
          bind:this={bgFileEl}
          onchange={onBgFileChosen}
          hidden
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
  /* 胶囊：非液态玻璃 = 原 M3 选中段样式（secondary-container 胶囊 inset 2px）
   * 位置/形变在两种模式下均由 rAF 弹簧驱动（v2.6：不再用 CSS transition，
   * 避免 JS 直控与过渡动画互相打架） */
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
  /* 液态玻璃：材质由 LiquidGlass.svelte 双层伪元素承载；
   * 热透镜（强折射+液态流动）由容器 .lg-dragging / 段 :active 触发 */
  :global(html.liquid-glass) .segment-pill {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  /* ---- 自定义背景卡片 ---- */
  .bg-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 16px;
    flex-wrap: wrap;
  }
  .bg-busy-hint {
    color: var(--md-on-surface-variant);
    font-size: 13px;
  }
  .bg-preview-wrap {
    flex-shrink: 0;
  }
  .bg-preview {
    display: block;
    width: 72px;
    height: 48px;
    object-fit: cover;
    border-radius: 8px;
    border: 1px solid var(--md-outline-variant);
    background: var(--md-surface-container-low);
  }
</style>
