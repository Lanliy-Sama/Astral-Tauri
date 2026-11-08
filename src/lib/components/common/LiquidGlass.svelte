<script>
  /**
   * 液态玻璃系统（光学参数对齐 glassium）—— 本 Svelte 5 组件自包含全部实现：
   *   1. <style> 的 :global 块承载全部光学材质（原独立 liquid-glass.css）：
   *      双层伪元素结构（::before 边缘透镜折射/色散 + ::after 中心磨砂/高光），
   *      按压/拖拽（--lg-press）、镜面高光跟随（--lg-spec/--lg-x/--lg-y）、
   *      开关玻璃管插值（--lg-on）。
   *   2. SVG 折射滤镜 defs：#lg-edge 静止态、#lg-edge-hot 按住/拖动态
   *      （feTurbulence + feDisplacementMap 只作用于边缘透镜层，
   *       中心磨砂层不带位移 —— 「边缘折射、中心纯净」）。
   *   3. JS：html.liquid-glass 根类随「软件设置 → 液态玻璃效果」开关切换；
   *      开启时委派 pointermove，把指针坐标写入 .lg-spec 元素的
   *      --lg-x/--lg-y（镜面高光跟随指针）。
   * 使用：App.svelte 渲染一次 <LiquidGlass />（无 UI，仅 defs + 全局样式 + 效果）。
   */
  import { sm } from '../../services/ServiceManager.svelte';

  $effect(() => {
    const on = sm.appSettingsState.liquidGlass;
    document.documentElement.classList.toggle('liquid-glass', on);
    if (!on) return;
    const move = (e) => {
      const el = e.target && e.target.closest ? e.target.closest('.lg-spec') : null;
      if (!el) return;
      const r = el.getBoundingClientRect();
      el.style.setProperty('--lg-x', (e.clientX - r.left) + 'px');
      el.style.setProperty('--lg-y', (e.clientY - r.top) + 'px');
    };
    document.addEventListener('pointermove', move);
    return () => {
      document.removeEventListener('pointermove', move);
      document.documentElement.classList.remove('liquid-glass');
    };
  });
</script>

<!-- 液态玻璃折射滤镜（对齐 glassium 光学模型）：
  feTurbulence + feDisplacementMap 只作用于边缘透镜层（:global 样式的 ::before，
  被 mask 限制在边缘带内），中心磨砂层不带位移 —— 与 glassium「边缘折射、中心纯净」一致。
  - #lg-edge：静止态，轻微几何位移（scale 9）
  - #lg-edge-hot：按住/拖拽态，强位移（scale 18）+ baseFrequency 持续漂移 = 液态流动 -->
<svg class="lg-defs" aria-hidden="true" focusable="false">
  <defs>
    <filter id="lg-edge" x="-20%" y="-20%" width="140%" height="140%" color-interpolation-filters="sRGB">
      <feTurbulence type="fractalNoise" baseFrequency="0.012 0.016" numOctaves="2" seed="7" result="lg-noise" />
      <feDisplacementMap in="SourceGraphic" in2="lg-noise" scale="9" xChannelSelector="R" yChannelSelector="G" />
    </filter>
    <filter id="lg-edge-hot" x="-30%" y="-30%" width="160%" height="160%" color-interpolation-filters="sRGB">
      <feTurbulence type="fractalNoise" baseFrequency="0.02 0.024" numOctaves="2" seed="7" result="lg-noise">
        <animate
          attributeName="baseFrequency"
          dur="2.8s"
          values="0.02 0.024;0.028 0.017;0.02 0.024"
          repeatCount="indefinite"
        />
      </feTurbulence>
      <feDisplacementMap in="SourceGraphic" in2="lg-noise" scale="18" xChannelSelector="R" yChannelSelector="G" />
    </filter>
  </defs>
</svg>

<style>
  .lg-defs {
    position: absolute;
    width: 0;
    height: 0;
    pointer-events: none;
  }

  /* ===== 液态玻璃 2.2 全局光学材质（原 src/liquid-glass.css，:global 全局作用） ===== */
  :global {
/* ============================================================
 * 液态玻璃 2.2 —— 光学参数对齐 glassium（WebGPU 液态玻璃渲染框架）
 * 的实测定标：docs/calibration.md + shaders/glass.wgsl.ts 常数。
 *
 * 本版关键改进：玻璃材质在【纯色/扁平背景】上也清晰可读。
 * 纯色背景上 blur/折射天然不可见，玻璃感必须来自静态光学结构：
 *   · 永久对角光泽（受光面→背光面，--lg-sheen）
 *   · 边缘亮环 + 顶缘斜面光（--lg-bevel）
 *   · 亮度抬升（--lg-bright）：玻璃面板比背景亮一档
 *   · 更饱和的双向渐变色调（--lg-tint）
 *
 * 双层结构（每个玻璃元素 = 自身 + 两个伪元素，均为 z:-1）：
 *  ::before「边缘透镜」—— mask 限制在边缘带内，采样【原始背景】：
 *    · url(#lg-edge) feTurbulence+feDisplacementMap 边缘折射
 *    · 饱和度 +35%、色散（左右边缘红/蓝微渐变）
 *    · 斜面光 + 全周亮环 + 上下主亮边
 *  ::after「中心磨砂」—— 中心镂空边缘，采样背景做 blur+saturate，
 *    再叠：永久对角光泽、镜面高光（--lg-x/--lg-y 跟随指针）、
 *    按压体积光/体光、色调
 *  元素自身：极细中性外线 + 投影（影子=背景压暗）
 *
 * 覆盖组件：开关（轨道+玻璃珠 thumb）、侧边栏选中框、分段控件
 * 选中块、底部导航选项卡、弹窗选项卡，以及【卡片】（md-card /
 * home-box）与【按钮】（FAB / filled / tonal / elevated / outlined）。
 *
 * 按压/拖拽（--lg-press 0→1，@property 注册平滑过渡）：
 *  · 透镜切 #lg-edge-hot：位移×2 + baseFrequency 漂移 = 液态流动
 *  · 体积光（按压点暖光）+ 体光 + 亮度提升
 * html.liquid-glass 由 MainScreen 按设置开关切换；关闭回落 M3 原样式。
 * ============================================================ */

/* ---- 可动画自定义属性（Chromium/WebView2 支持 @property） ---- */
@property --lg-spec {
  syntax: '<number>';
  inherits: true;
  initial-value: 0;
}
@property --lg-press {
  syntax: '<number>';
  inherits: true;
  initial-value: 0;
}
@property --lg-on {
  syntax: '<number>';
  inherits: true;
  initial-value: 0;
}

/* ---- 全局光学参数（继承到所有玻璃元素；暗色模式自动收敛） ----
 * 2.3 增强档：光泽/斜面/饱和略升，色散改为全周棱镜彩环（见 ::before） */
html.liquid-glass {
  --lg-sheen: 0.34; /* 永久对角光泽强度（受光面白） */
  --lg-bevel: 0.55; /* 顶缘斜面光强度 */
}
html.liquid-glass[data-theme-mode='dark'] {
  --lg-sheen: 0.2;
  --lg-bevel: 0.34;
}

/* ---- 玻璃材质基座：元素自身（外线 + 投影 + 状态变量） ----
 * 材质同时应用于：标记 .lg-glass 的组件、卡片（md-card / home-box）、
 * 按钮（FAB / filled / tonal / elevated / outlined）。 */
html.liquid-glass .lg-glass,
html.liquid-glass .md-card,
html.liquid-glass .home-box:not(.borderless),
html.liquid-glass .md-fab,
html.liquid-glass .md-filled-btn,
html.liquid-glass .md-tonal-btn,
html.liquid-glass .md-elevated-btn,
html.liquid-glass .md-outlined-btn,
html.liquid-glass .md-menu,
html.liquid-glass .list-user-card,
html.liquid-glass .all-card,
html.liquid-glass .mini-card,
html.liquid-glass .md-dialog {
  --lg-edge: 18px; /* 边缘折射带宽度（各组件按尺寸覆盖） */
  --lg-blur: 9px; /* 中心磨砂半径 */
  --lg-spec-r: 110px; /* 镜面高光半径 */
  --lg-bright: 1; /* 中心亮度抬升（纯色背景可读性的关键） */
  --lg-tint: transparent; /* 中心色调（渐变或颜色） */
  --lg-tint-edge: transparent; /* 边缘透镜上的附加色调 */
  position: relative;
  isolation: isolate; /* 元素自成 backdrop root：伪元素不被自身背景干扰 */
  background: transparent; /* 色调全部由伪元素承载 */
  border: 1px solid color-mix(in srgb, var(--md-on-surface) 15%, transparent);
  /* 全周亮环从 ::before 移回本体：::before 挂着扭曲滤镜，环会被一起扭成波浪；
   * 本体不受 filter 影响，保证卡片外形规则挺括，只有边缘带内的画面折射 */
  box-shadow:
        inset 0 0 0 1px rgb(255 255 255 / calc(0.44 + 0.3 * var(--lg-press))),
    inset 0 0 0.5px 0.5px rgb(0 0 0 / 0.1),
    inset 0 3px 2px rgb(255 255 255 / 0.9),
    inset 0 -3px 2px rgb(255 255 255 / 0.7),
    inset 2px 0 2px rgb(255 255 255 / 0.38),
    inset -2px 0 2px rgb(255 255 255 / 0.38),
    0 1px 2px rgba(0, 0, 0, 0.12),
    0 4px 14px rgba(0, 0, 0, 0.12);
  transition:
    --lg-spec 320ms cubic-bezier(0.2, 0, 0, 1),
    --lg-press 180ms ease,
    --lg-on 300ms cubic-bezier(0.2, 0, 0, 1),
    border-color 240ms ease,
    box-shadow 240ms ease,
    transform 150ms ease;
}

/* ---- 底层：边缘透镜（折射 · 色散 · 斜面光 · 全周亮环） ---- */
html.liquid-glass .lg-glass::before,
html.liquid-glass .md-card::before,
html.liquid-glass .home-box:not(.borderless)::before,
html.liquid-glass .md-fab::before,
html.liquid-glass .md-filled-btn::before,
html.liquid-glass .md-tonal-btn::before,
html.liquid-glass .md-elevated-btn::before,
html.liquid-glass .md-outlined-btn::before,
html.liquid-glass .md-menu::before,
html.liquid-glass .list-user-card,
html.liquid-glass .all-card,
html.liquid-glass .mini-card,
html.liquid-glass .md-dialog::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  z-index: -1;
  pointer-events: none;
  /* 边缘扭曲折射：backdrop-filter 的 SVG url() 引用 Chromium 不支持（且本机
   * 带 mask 的伪元素 backdrop 全不渲染），改用普通 filter —— 对边缘带内的
   * 【背景图副本】做湍流位移扭曲（#lg-edge），mask 裁回边带 = 真折射。 */
  filter: url(#lg-edge);
  background:
    /* 色散（棱镜彩环）2.3：四向边缘染色 —— 左红右蓝为主，
     * 上下补充暖橙/冷青，彩色光沿整圈轮廓连续可辨；
     * 叠在斜面光之上，避免被白光洗掉 */
    linear-gradient(to right, rgb(255 72 72 / 0.28), transparent 36%),
    linear-gradient(to left, rgb(72 138 255 / 0.28), transparent 36%),
    linear-gradient(to bottom, rgb(255 122 82 / 0.16), transparent 30%),
    linear-gradient(to top, rgb(82 190 255 / 0.16), transparent 30%),
    /* 斜面光：顶缘受光、底缘地面反光（glassium bevel） */
    linear-gradient(to bottom, rgb(255 255 255 / var(--lg-bevel)), transparent 28%),
    linear-gradient(to top, rgb(255 255 255 / calc(var(--lg-bevel) * 0.42)), transparent 32%),
    var(--lg-tint-edge),
    /* 背景图副本（fixed = 相对视口，与 .main-screen 的 cover/center 对齐）：
     * 经 #lg-edge 位移后呈现边缘扭曲折射；无自定义背景时该层为空 */
    var(--astral-bg-image, none) fixed center / cover no-repeat;
  /* 边缘带 mask（四层 add 合成 = 四边框；显式 size/repeat 防合成层平铺） */
  -webkit-mask-image:
    linear-gradient(to right, #000 0, transparent var(--lg-edge)),
    linear-gradient(to left, #000 0, transparent var(--lg-edge)),
    linear-gradient(to bottom, #000 0, transparent var(--lg-edge)),
    linear-gradient(to top, #000 0, transparent var(--lg-edge));
  -webkit-mask-size: 100% 100%;
  -webkit-mask-repeat: no-repeat;
  -webkit-mask-composite: source-over;
  mask-image:
    linear-gradient(to right, #000 0, transparent var(--lg-edge)),
    linear-gradient(to left, #000 0, transparent var(--lg-edge)),
    linear-gradient(to bottom, #000 0, transparent var(--lg-edge)),
    linear-gradient(to top, #000 0, transparent var(--lg-edge));
  mask-size: 100% 100%;
  mask-repeat: no-repeat;
  mask-composite: add;
}

/* ---- 上层：中心磨砂（blur · 光泽 · 高光 · 按压光效 · 色调） ---- */
html.liquid-glass .lg-glass::after,
html.liquid-glass .md-card::after,
html.liquid-glass .home-box:not(.borderless)::after,
html.liquid-glass .md-fab::after,
html.liquid-glass .md-filled-btn::after,
html.liquid-glass .md-tonal-btn::after,
html.liquid-glass .md-elevated-btn::after,
html.liquid-glass .md-outlined-btn::after,
html.liquid-glass .md-menu::after,
html.liquid-glass .list-user-card,
html.liquid-glass .all-card,
html.liquid-glass .mini-card,
html.liquid-glass .md-dialog::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  z-index: -1;
  pointer-events: none;
  -webkit-backdrop-filter: blur(var(--lg-blur)) saturate(1.7) brightness(calc(var(--lg-bright) - 0.06 * var(--lg-press)));
  backdrop-filter: blur(var(--lg-blur)) saturate(1.7) brightness(calc(var(--lg-bright) - 0.06 * var(--lg-press)));
  background:
    /* 按压变暗层：玻璃被压入表面 → 变厚变暗（浅色背景下清晰可读的按压反馈）
     * 注：浅色背景上“提亮”会裁剪到白色而不可见，Apple 液态玻璃按压缩果同样是变暗 */
    linear-gradient(
      to bottom,
      rgb(0 0 0 / calc(0.08 * var(--lg-press))),
      rgb(0 0 0 / calc(0.05 * var(--lg-press))) 62%,
      rgb(0 0 0 / calc(0.07 * var(--lg-press)))
    ),
    /* 永久对角光泽：受光面白 → 背光面微暗（纯色背景上的玻璃厚度感） */
    linear-gradient(
      155deg,
      rgb(255 255 255 / var(--lg-sheen)),
      rgb(255 255 255 / calc(var(--lg-sheen) * 0.22)) 36%,
      transparent 50%,
      rgb(0 0 0 / 0.028) 84%
    ),
    /* 静息光：顶面反光 + 底部地面反射（--lg-rest=0 时完全清除，卡片中心全透） */
    linear-gradient(
      to bottom,
      rgb(255 255 255 / calc(0.2 * var(--lg-rest, 1))),
      transparent 34% 62%,
      rgb(255 255 255 / calc(0.11 * var(--lg-rest, 1)))
    ),
    /* 镜面反射：跟随指针的实时高光（悬停 --lg-spec） */
    radial-gradient(
      var(--lg-spec-r) at var(--lg-x, 50%) var(--lg-y, 50%),
      rgb(255 255 255 / calc(0.4 * var(--lg-spec))),
      rgb(255 255 255 / calc(0.08 * var(--lg-spec))) 45%,
      transparent 72%
    ),
    /* 按压体积光：按压点暖光（glassium press light；叠加在变暗层上 → 可见的暖色光斑） */
    radial-gradient(
      calc(var(--lg-spec-r) + 60px) at var(--lg-x, 50%) var(--lg-y, 50%),
      rgb(255 214 130 / calc(0.3 * var(--lg-press))),
      transparent 72%
    ),
    /* 体光：按住时顶暗底亮（BODY_SHADE 0.047 / BODY_LIGHT 0.055） */
    linear-gradient(
      to bottom,
      rgb(0 0 0 / calc(0.05 * var(--lg-press))),
      transparent 30% 70%,
      rgb(255 255 255 / calc(0.09 * var(--lg-press)))
    ),
    var(--lg-tint);
  /* 中心镂空（边缘露出透镜；与透镜带留 25%→100% 重叠做宽羽化交叉渐变：
   * 透镜淡出的同时中心淡入，避免硬边。四层 mask 取【交集 intersect】
   * 才是“中心实、四边空”的环形；此前误用 add（并集）→ 整个元素被盖实，
   * 边缘透镜被埋住，色散/折射带完全不可见 —— 2.2 根因修复 */
  -webkit-mask-image:
    linear-gradient(to right, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge)),
    linear-gradient(to left, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge)),
    linear-gradient(to bottom, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge)),
    linear-gradient(to top, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge));
  -webkit-mask-size: 100% 100%;
  -webkit-mask-repeat: no-repeat;
  -webkit-mask-composite: source-in;
  mask-image:
    linear-gradient(to right, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge)),
    linear-gradient(to left, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge)),
    linear-gradient(to bottom, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge)),
    linear-gradient(to top, transparent calc(var(--lg-edge) * 0.25), #000 var(--lg-edge));
  mask-size: 100% 100%;
  mask-repeat: no-repeat;
  mask-composite: intersect;
}

/* ---- 绝对定位的玻璃元素（基座 position:relative 会盖过组件样式，这里显式恢复） ---- */
html.liquid-glass .md-switch .thumb,
html.liquid-glass .nav-items > .selection-indicator,
html.liquid-glass .md-segmented .segment-pill,
html.liquid-glass .bottom-nav .selection-pill,
html.liquid-glass .servers-fab,
html.liquid-glass .logs-fab,
html.liquid-glass .mw-fab {
  position: absolute; /* absolute 同样构成伪元素的 containing block，玻璃正确铺满 */
}

/* ---- 弹出菜单：必须保持 fixed（App.svelte .md-menu） ----
 * 玻璃基座的 position:relative 会把弹出菜单拉回文档流：
 * 状态栏主题/语言菜单打开时菜单变成 flex 子项，把主题选择、
 * 色彩调整、语言设置图标整体挤出原位（错位 bug 的根因）。
 * fixed 元素自身仍是伪元素的 containing block，玻璃照常铺满。 */
html.liquid-glass .md-menu {
  position: fixed;
}

/* ---- 悬停：镜面高光亮起 + 投影加深（玻璃浮起） ---- */
html.liquid-glass .lg-glass:hover,
html.liquid-glass .lg-glass.lg-hover,
html.liquid-glass .md-card:hover,
html.liquid-glass .home-box:not(.borderless):hover,
html.liquid-glass .md-fab:hover,
html.liquid-glass .md-filled-btn:hover,
html.liquid-glass .md-tonal-btn:hover,
html.liquid-glass .md-elevated-btn:hover,
html.liquid-glass .md-outlined-btn:hover {
  --lg-spec: 1;
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.14),
    0 6px 20px rgba(0, 0, 0, 0.16);
}

/* ---- 按住：透镜切强折射+液态流动滤镜，光效走 --lg-press ----
 * 卡片/home-box 用 :not(:has(:active)) 限定【直接按压卡片本身】：
 * :active 会沿祖先链传播（按内部开关/按钮时卡片也 :active），
 * 若不排除会把整张卡片罩进按压暖光 —— 光效泄漏到未交互控件 */
html.liquid-glass .lg-glass:active,
html.liquid-glass .lg-glass.lg-pressed,
html.liquid-glass .lg-dragging .lg-glass,
html.liquid-glass .md-card:active:not(:has(:active)),
html.liquid-glass .home-box:not(.borderless):active:not(:has(:active)),
html.liquid-glass .md-fab:active,
html.liquid-glass .md-filled-btn:active,
html.liquid-glass .md-tonal-btn:active,
html.liquid-glass .md-elevated-btn:active,
html.liquid-glass .md-outlined-btn:active {
  --lg-press: 1;
}
html.liquid-glass .lg-glass:active::before,
html.liquid-glass .lg-glass.lg-pressed::before,
html.liquid-glass .lg-dragging .lg-glass::before,
html.liquid-glass .md-card:active:not(:has(:active))::before,
html.liquid-glass .home-box:not(.borderless):active:not(:has(:active))::before,
html.liquid-glass .md-fab:active::before,
html.liquid-glass .md-filled-btn:active::before,
html.liquid-glass .md-tonal-btn:active::before,
html.liquid-glass .md-elevated-btn:active::before,
html.liquid-glass .md-outlined-btn:active::before {
  -webkit-backdrop-filter: url(#lg-edge-hot) blur(1px) saturate(1.55) brightness(1.1);
  backdrop-filter: url(#lg-edge-hot) blur(1px) saturate(1.55) brightness(1.1);
}

/* ---- 按压回弹：按钮/卡片轻微收缩（选中块/滑块的位置由 JS 内联 transform 驱动，不参与） ---- */
html.liquid-glass .md-fab:active,
html.liquid-glass .md-filled-btn:active,
html.liquid-glass .md-tonal-btn:active,
html.liquid-glass .md-elevated-btn:active,
html.liquid-glass .md-outlined-btn:active {
  transform: scale(0.97);
}

/* ---- 选项/选项卡按压的局部反馈：指针下即时渗色压痕 ----
 * 选中胶囊（pill/indicator）负责“选了谁”，这里负责“按在哪”：
 * 按住的瞬间在指针下的选项上出现主色压痕，松手随胶囊滑动接续。
 * 分段拖动中（.lg-dragging）：改由 .segment.hovered 承接
 * （胶囊连续跟随指针，原段 :active 压度不再有意义）。 */
html.liquid-glass .nav-items:not(.lg-dragging) .nav-item:active,
html.liquid-glass .md-segmented:not(.lg-dragging) .segment:active,
html.liquid-glass .bottom-nav .bottom-nav-item:active {
  background: color-mix(in srgb, var(--md-primary) 10%, transparent);
  border-radius: 999px;
}
html.liquid-glass .nav-items:not(.lg-dragging) .nav-item:active {
  border-radius: 12px;
}
/* 分段拖动中：指针下的段即时高亮（胶囊连续跟随，段反馈“将落在哪”） */
html.liquid-glass .md-segmented.lg-dragging .segment.hovered {
  background: color-mix(in srgb, var(--md-primary) 10%, transparent);
  border-radius: 999px;
}
html.liquid-glass .md-segmented.lg-dragging {
  cursor: grabbing;
}


/* ============================================================
 * 卡片：磨砂玻璃板（md-card / home-box）
 * 纯色背景上：亮度抬升 + 对角光泽 + 亮环 + 双向渐变色调
 * ============================================================ */
/* 成员页面板：让背景图直达（原实色 --md-surface 遮挡了下半页） */
html.liquid-glass .user-page {
  background: transparent;
}
html.liquid-glass .user-page .scroll {
  background: transparent;
}

html.liquid-glass .md-card,
html.liquid-glass .home-box:not(.borderless) {
  --lg-edge: 22px;
  --lg-blur: 13px;
  --lg-spec-r: 150px;
  --lg-bright: 1.06;
  /* 中心完全透明：镜面反光近零、静息光清零（见 ::after 的 --lg-rest），
   * 卡内只留边缘扭曲折射带与一丝玻璃反光 */
  --lg-sheen: 0.05;
  --lg-rest: 0;
  /* 中心完全透明（无表面色无白雾）：卡片 = 边缘折射带 + 玻璃反光，
   * 中心直通背景。 */
  --lg-tint: transparent;
}
/* hover 描边回归组件语义（HomeBox.hovered = primary） */
html.liquid-glass .home-box:not(.borderless).hovered {
  border-color: var(--md-primary);
}

/* ============================================================
 * 悬浮面板：菜单 / 对话框 —— 真实折射的最佳舞台
 * 它们浮在任意内容之上：背景被边缘透镜弯折、中心磨砂。
 * 色调取近实色 surface（96%+）：Chromium 在 fixed scrim 内不渲染
 * backdrop-filter（blur 失效），可读性由 tint 承担；玻璃感保留在
 * 边缘折射带 + 亮边 + 光泽。
 * ============================================================ */
html.liquid-glass .md-menu,
html.liquid-glass .list-user-card,
html.liquid-glass .all-card,
html.liquid-glass .mini-card,
html.liquid-glass .md-dialog {
  --lg-edge: 20px;
  --lg-blur: 16px;
  --lg-spec-r: 160px;
  --lg-bright: 1.07;
  --lg-tint:
    linear-gradient(
      160deg,
      color-mix(in srgb, var(--md-surface-container) 96%, rgb(255 255 255 / 0.5)),
      color-mix(in srgb, var(--md-surface-container) 99%, rgb(255 255 255 / 0.28)) 55%,
      color-mix(in srgb, var(--md-surface-container) 97%, rgb(255 255 255 / 0.4))
    );
}

/* v2.5 —— 悬浮面板色散改为【上下镜像对称】：
 * 基座的四向色散（左红右蓝）在小面板上读作左右色温不均，
 * 整块玻璃像被染歪/倾斜；对角分布仍残留倾斜感（v2.5 首版试过）。
 * 最终方案：只保留上缘暖色棱镜 + 下缘冷色棱镜（左右严格镜像），
 * 横向零失衡 —— 光像从玻璃顶面射入、底面折射出，物理上说得通，
 * 棱镜彩感保留，面板端正不歪。 */
html.liquid-glass .md-menu::before,
html.liquid-glass .list-user-card,
html.liquid-glass .all-card,
html.liquid-glass .mini-card,
html.liquid-glass .md-dialog::before {
  background:
    /* 上下棱镜色散（左右镜像对称）：顶缘暖橙、底缘冷青 */
    linear-gradient(to bottom, rgb(255 110 70 / 0.26), transparent 40%),
    linear-gradient(to top, rgb(70 160 255 / 0.26), transparent 40%),
    /* 斜面光：与基座一致（顶缘受光、底缘地面反光） */
    linear-gradient(to bottom, rgb(255 255 255 / var(--lg-bevel)), transparent 28%),
    linear-gradient(to top, rgb(255 255 255 / calc(var(--lg-bevel) * 0.42)), transparent 32%),
    var(--lg-tint-edge),
    var(--astral-bg-image, none) fixed center / cover no-repeat;
  filter: url(#lg-edge);
}

/* ============================================================
 * 按钮：彩色玻璃（FAB / filled / tonal / elevated / outlined）
 * 颜色由 --lg-tint 承载（伪元素覆盖元素背景），
 * 光学结构与卡片一致：光泽 + 亮环 + 按压流光
 * ============================================================ */
html.liquid-glass .md-fab {
  --lg-edge: 14px;
  --lg-blur: 9px;
  --lg-spec-r: 90px;
  --lg-bright: 1.03;
  --lg-tint:
    linear-gradient(
      155deg,
      color-mix(in srgb, var(--md-primary-container) 34%, rgb(255 255 255 / 0.4)),
      color-mix(in srgb, var(--md-primary-container) 50%, rgb(255 255 255 / 0.1)) 58%,
      color-mix(in srgb, var(--md-primary-container) 30%, rgb(255 255 255 / 0.24))
    );
}
html.liquid-glass .md-filled-btn {
  --lg-edge: 10px;
  --lg-blur: 7px;
  --lg-spec-r: 70px;
  --lg-bright: 1.02;
  --lg-tint:
    linear-gradient(
      155deg,
      color-mix(in srgb, var(--md-primary) 40%, rgb(255 255 255 / 0.38)),
      color-mix(in srgb, var(--md-primary) 56%, rgb(255 255 255 / 0.08)) 58%,
      color-mix(in srgb, var(--md-primary) 32%, rgb(255 255 255 / 0.22))
    );
}
html.liquid-glass .md-tonal-btn {
  --lg-edge: 10px;
  --lg-blur: 7px;
  --lg-spec-r: 70px;
  --lg-bright: 1.03;
  --lg-tint:
    linear-gradient(
      155deg,
      color-mix(in srgb, var(--md-secondary-container) 36%, rgb(255 255 255 / 0.4)),
      color-mix(in srgb, var(--md-secondary-container) 52%, rgb(255 255 255 / 0.1)) 58%,
      color-mix(in srgb, var(--md-secondary-container) 30%, rgb(255 255 255 / 0.24))
    );
}
html.liquid-glass .md-elevated-btn,
html.liquid-glass .md-outlined-btn {
  --lg-edge: 10px;
  --lg-blur: 7px;
  --lg-spec-r: 70px;
  --lg-bright: 1.04;
  --lg-tint:
    linear-gradient(
      155deg,
      rgb(255 255 255 / 0.16),
      rgb(255 255 255 / 0.03) 45%,
      rgb(0 0 0 / 0.01)
    );
}
/* 禁用态：玻璃褪为哑光淡板（颜色/阴影全部收敛） */
html.liquid-glass .md-filled-btn:disabled,
html.liquid-glass .md-tonal-btn:disabled,
html.liquid-glass .md-elevated-btn:disabled,
html.liquid-glass .md-outlined-btn:disabled,
html.liquid-glass .md-fab:disabled {
  --lg-tint:
    linear-gradient(
      155deg,
      rgb(255 255 255 / 0.12),
      transparent 60%
    );
  border-color: color-mix(in srgb, var(--md-on-surface) 10%, transparent);
  box-shadow: none;
}
/* 连接按钮宽度伸缩动画保留（基座 transition 不含 width） */
html.liquid-glass .md-fab.connect-fab {
  transition:
    width 200ms cubic-bezier(0, 0, 0.2, 1),
    --lg-spec 320ms cubic-bezier(0.2, 0, 0, 1),
    --lg-press 180ms ease,
    --lg-on 300ms cubic-bezier(0.2, 0, 0, 1),
    border-color 240ms ease,
    box-shadow 240ms ease,
    transform 150ms ease;
}

/* ============================================================
 * 组件：开关（52×32 轨道 = 主色玻璃管，thumb = 玻璃珠）
 * 2.2 强化：ON = 饱和半透明主色玻璃管（受光面亮→主体深→底部反光）
 * + 顶弧高光 + 内亮环 + 主色氛围光晕；OFF = 白磨砂玻璃。
 * 玻璃珠：镜面热点 + 上部受光 + 底缘反光 + ON 时底部透出管色。
 * 色散描边在小元素上提高到 0.3 才可见。
 * ============================================================ */
html.liquid-glass .md-switch {
  --lg-edge: 7px;
  --lg-blur: 6px;
  --lg-spec-r: 70px;
  --lg-bright: 1.03;
  /* 主体色调：--lg-on 在冷灰白磨砂玻璃 ↔ 饱和主色玻璃管之间平滑插值
   * （OFF 用冷灰而非纯白：浅色卡片上纯白会“洗白”，冷灰保对比度） */
  --lg-tint:
    linear-gradient(
      168deg,
      color-mix(in srgb, var(--md-primary) calc(74% * var(--lg-on)), rgb(150 158 172 / 0.52)),
      color-mix(in srgb, var(--md-primary) calc(91% * var(--lg-on)), rgb(105 115 133 / 0.36)) 46%,
      color-mix(in srgb, var(--md-primary) calc(80% * var(--lg-on)), rgb(132 140 155 / 0.48))
    );
  /* 亮环移回本体（同卡片）：::before 只承担扭曲折射带 */
  box-shadow:
    inset 0 0 0 1px rgb(255 255 255 / calc(0.4 + 0.3 * var(--lg-on) + 0.2 * var(--lg-press))),
    inset 0 0 0.5px 0.5px rgb(0 0 0 / calc(0.14 + 0.1 * (1 - var(--lg-on)))),
    inset 0 2px 1.5px rgb(255 255 255 / 0.98),
    inset 0 -2.5px 2px rgb(255 255 255 / 0.6),
    inset 2px 0 2px rgb(255 255 255 / 0.42),
    inset -2px 0 2px rgb(255 255 255 / 0.42);
  /* 外线：OFF 中性细线 → ON 主色描边（玻璃管透出管色） */
  border-color: color-mix(
    in srgb,
    var(--md-primary) calc(44% * var(--lg-on)),
    color-mix(in srgb, var(--md-on-surface) 16%, transparent)
  );
  /* 悬浮投影 + ON 主色氛围光晕（玻璃管向外发光） */
  box-shadow:
    0 1px 2px rgb(0 0 0 / 0.16),
    0 3px 10px rgb(0 0 0 / 0.13),
    0 2px 16px color-mix(in srgb, var(--md-primary) calc(36% * var(--lg-on)), transparent);
}
html.liquid-glass .md-switch.on {
  --lg-on: 1;
}
/* 轨道边缘透镜：玻璃管弧光（顶受光/底反弹）+ 色散描边
 * （色散梯度长度对齐边缘带宽度：红/蓝集中在最外缘再衰减到 0） */
html.liquid-glass .md-switch::before {
  background:
    linear-gradient(to right, rgb(255 85 85 / 0.68), transparent 20%),
    linear-gradient(to left, rgb(85 145 255 / 0.68), transparent 20%),
    /* 上下缘的弱色散：让彩边沿整条轮廓连续（横向为主、纵向为辅） */
    linear-gradient(to bottom, rgb(255 110 80 / 0.18), transparent 18%),
    linear-gradient(to top, rgb(80 185 255 / 0.18), transparent 18%),
    linear-gradient(to bottom, rgb(255 255 255 / var(--lg-bevel)), transparent 24%),
    linear-gradient(to top, rgb(255 255 255 / calc(0.42 * var(--lg-bevel))), transparent 36%),
    var(--astral-bg-image, none) fixed center / cover no-repeat;
  /* 开关轨道同样做边缘扭曲折射（与卡片同一滤镜/同一背景对齐） */
  filter: url(#lg-edge);
}
/* 轨道中心磨砂：管内体光（顶受光/底反弹）+ 指针高光 + 按压光效 + 主体色 */
html.liquid-glass .md-switch::after {
  background:
    linear-gradient(
      to bottom,
      rgb(0 0 0 / calc(0.09 * var(--lg-press))),
      rgb(0 0 0 / calc(0.05 * var(--lg-press))) 62%,
      rgb(0 0 0 / calc(0.08 * var(--lg-press)))
    ),
    radial-gradient(
      calc(var(--lg-spec-r) + 60px) at var(--lg-x, 50%) var(--lg-y, 50%),
      rgb(255 214 130 / calc(0.22 * var(--lg-press))),
      transparent 72%
    ),
    radial-gradient(
      var(--lg-spec-r) at var(--lg-x, 50%) var(--lg-y, 50%),
      rgb(255 255 255 / calc(0.5 * var(--lg-spec))),
      transparent 70%
    ),
    linear-gradient(
      to bottom,
      rgb(255 255 255 / calc(0.55 * var(--lg-bevel))),
      transparent 40% 62%,
      rgb(255 255 255 / calc(0.3 * var(--lg-bevel)))
    ),
    /* ON 透光核心：光从管内透出（背亮玻璃管） */
    radial-gradient(
      62% 92% at 50% 46%,
      rgb(255 255 255 / calc(0.24 * var(--lg-on))),
      transparent 72%
    ),
    var(--lg-tint);
}
/* thumb：玻璃珠（16/24px 上边缘带已无意义 —— 解除 mask，双层全幅铺满） */
html.liquid-glass .md-switch .thumb {
  --lg-edge: 5px;
  --lg-blur: 2px;
  --lg-spec-r: 42px;
  position: absolute;
  background: transparent;
  border: none;
  box-shadow: none;
  /* 玻璃珠悬浮于管上：接触阴影（近实 + 远虚）+ ON 时主色透光晕 */
  filter:
    drop-shadow(0 2px 6px rgb(0 0 0 / 0.45))
    drop-shadow(0 0.5px 1.5px rgb(0 0 0 / 0.28))
    drop-shadow(0 0 9px color-mix(in srgb, var(--md-primary) calc(46% * var(--lg-on)), transparent));
}
html.liquid-glass .md-switch .thumb::before,
html.liquid-glass .md-switch .thumb::after {
  -webkit-mask-image: none;
  mask-image: none;
}
/* 珠底：致密玻璃核 + 底部折射管色（ON 时主色透入珠底） */
html.liquid-glass .md-switch .thumb::before {
  background:
    linear-gradient(
      to top,
      color-mix(in srgb, var(--md-primary) calc(55% * var(--lg-on)), rgb(125 135 155 / 0.45)),
      transparent 58%
    ),
    radial-gradient(
      circle at 50% 40%,
      rgb(255 255 255 / 0.94),
      rgb(210 218 232 / 0.52) 66%,
      rgb(128 140 163 / 0.55)
    );
  /* 折射暗线：玻璃珠边缘的 1px 深色环（OFF 时加强 —— 浅底上的轮廓救星） */
  box-shadow:
    inset 0 0 0 1px rgb(62 74 94 / calc(0.24 + 0.2 * (1 - var(--lg-on)))),
    inset 0 -1.5px 2px rgb(255 255 255 / 0.65),
    inset 0 1px 1.5px rgb(0 0 0 / 0.1);
}
/* 珠面：镜面热点（小而亮）+ 上部受光 + 底缘反光 + 指针跟随高光 */
html.liquid-glass .md-switch .thumb::after {
  background:
    radial-gradient(
      4px 4px at 33% 26%,
      rgb(255 255 255 / 0.95),
      rgb(255 255 255 / 0.28) 62%,
      transparent 100%
    ),
    linear-gradient(to bottom, rgb(255 255 255 / 0.65), transparent 44%),
    linear-gradient(to top, rgb(255 255 255 / 0.48), transparent 32%),
    radial-gradient(
      var(--lg-spec-r) at var(--lg-x, 50%) var(--lg-y, 36%),
      rgb(255 255 255 / calc(0.45 * var(--lg-spec))),
      transparent 66%
    ),
    radial-gradient(
      calc(var(--lg-spec-r) + 40px) at var(--lg-x, 50%) var(--lg-y, 36%),
      rgb(255 214 130 / calc(0.22 * var(--lg-press))),
      transparent 70%
    );
}
/* 修复 app.css 悬停/禁用规则把玻璃珠退化为扁平色块 */
html.liquid-glass .md-switch:not(:disabled):hover.on .thumb,
html.liquid-glass .md-switch:not(:disabled):hover:not(.on) .thumb,
html.liquid-glass .md-switch:disabled .thumb,
html.liquid-glass .md-switch.on:disabled .thumb {
  background: transparent;
}
/* 拖动：透镜切强折射 + 液态流动（--lg-press 全套光效持续） */
html.liquid-glass .md-switch.dragging {
  --lg-press: 1;
}
html.liquid-glass .md-switch.dragging::before {
  /* 拖动中：动画噪声 + 强位移（#lg-edge-hot）—— 边缘"沸腾"折射 */
  filter: url(#lg-edge-hot);
}
/* 拖动 thumb 时：位置由 JS 内联控制，禁用过渡 */
html.liquid-glass .md-switch.dragging .thumb {
  transition: none;
}
html.liquid-glass .md-switch:not(.dragging) .thumb {
  /* 液态弹性：left/width 过冲回落（glassium 动态变形；2.4 软化过冲 → 更优雅） */
  transition:
    left 0.38s cubic-bezier(0.22, 1.2, 0.36, 1),
    width 0.38s cubic-bezier(0.22, 1.2, 0.36, 1),
    height 0.38s cubic-bezier(0.22, 1.2, 0.36, 1),
    filter 0.2s ease;
}
/* 禁用态：整体降不透明度，玻璃仍保留 */
html.liquid-glass .md-switch:disabled {
  opacity: 0.55;
}

/* ============================================================
 * 组件：侧边栏选中框（拖一拖滑块）—— 液态玻璃 + 弹簧跟随
 * 纯色背景上的强可读设计：主色玻璃色调 + 亮环 + 主色投影
 * ============================================================ */
html.liquid-glass .nav-items {
  position: relative;
}
html.liquid-glass .nav-items > .selection-indicator {
  --lg-edge: 16px;
  --lg-blur: 9px;
  --lg-spec-r: 90px;
  --lg-bright: 1.06;
  /* 主色玻璃：受光面近白，背光面主色加深 —— 与默认扁平选中色明显区分 */
  --lg-tint:
    linear-gradient(
      155deg,
      color-mix(in srgb, var(--md-primary) 24%, rgb(255 255 255 / 0.5)),
      color-mix(in srgb, var(--md-primary) 36%, rgb(255 255 255 / 0.2)) 52%,
      color-mix(in srgb, var(--md-primary) 20%, rgb(255 255 255 / 0.42))
    );
  background: transparent;
  border-color: color-mix(in srgb, var(--md-primary) 30%, transparent);
  /* 主色柔光投影：选中框从侧边栏“浮起” */
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.14),
    0 5px 16px color-mix(in srgb, var(--md-primary) 32%, transparent),
    0 0 0 1px color-mix(in srgb, var(--md-primary) 18%, transparent);
  /* 位置由 rAF 弹簧驱动：禁用 CSS 过渡 */
  transition: none;
  animation: lg-morph 9s ease-in-out infinite;
}
html.liquid-glass .nav-items > .selection-indicator.dragging {
  animation: none; /* 拖拽时停止呼吸，让拉伸形变主导 */
}
@keyframes lg-morph {
  0%, 100% { border-radius: 16px 14px 15px 14px; }
  33% { border-radius: 14px 16px 14px 15px; }
  66% { border-radius: 15px 14px 16px 14px; }
}

/* ============================================================
 * 组件：分段控件选中块（滑动玻璃胶囊）
 * ============================================================ */
html.liquid-glass .md-segmented {
  position: relative;
}
html.liquid-glass .md-segmented .segment-pill {
  --lg-edge: 11px;
  --lg-blur: 7px;
  --lg-spec-r: 80px;
  --lg-bright: 1.05;
  --lg-tint:
    linear-gradient(
      155deg,
      color-mix(in srgb, var(--md-primary) 16%, rgb(255 255 255 / 0.52)),
      color-mix(in srgb, var(--md-secondary-container) 62%, rgb(255 255 255 / 0.26)) 55%,
      color-mix(in srgb, var(--md-primary) 10%, rgb(255 255 255 / 0.4))
    );
  /* v2.6：transform 不参与过渡 —— 胶囊位置/液态形变由 rAF 弹簧逐帧直控
   * （基底 .lg-glass 的 transform 150ms 会让拖拽跟随/回弹滞后）；
   * 光影材质过渡（--lg-spec/--lg-press/border/shadow）保留。 */
  transition-property: --lg-spec, --lg-press, --lg-on, border-color, box-shadow;
}
/* 按住任意段：胶囊进入按压态（液态流动 + 体积光）；拖动中由 .lg-dragging 延续 */
html.liquid-glass .md-segmented:has(.segment:active) .segment-pill,
html.liquid-glass .md-segmented.lg-dragging .segment-pill {
  --lg-press: 1;
}
html.liquid-glass .md-segmented:has(.segment:active) .segment-pill::before,
html.liquid-glass .md-segmented.lg-dragging .segment-pill::before {
  -webkit-backdrop-filter: url(#lg-edge-hot) blur(1px) saturate(1.55) brightness(1.1);
  backdrop-filter: url(#lg-edge-hot) blur(1px) saturate(1.55) brightness(1.1);
}

/* ============================================================
 * 组件：底部导航选中块（移动端选项卡）
 * ============================================================ */
html.liquid-glass .bottom-nav {
  position: relative;
}
html.liquid-glass .bottom-nav .selection-pill {
  --lg-edge: 12px;
  --lg-blur: 8px;
  --lg-spec-r: 80px;
  --lg-bright: 1.05;
  --lg-tint:
    linear-gradient(
      155deg,
      color-mix(in srgb, var(--md-primary) 14%, rgb(255 255 255 / 0.5)),
      color-mix(in srgb, var(--md-secondary-container) 58%, rgb(255 255 255 / 0.28)) 55%,
      color-mix(in srgb, var(--md-primary) 9%, rgb(255 255 255 / 0.4))
    );
}
html.liquid-glass .bottom-nav:has(.bottom-nav-item:active) .selection-pill {
  --lg-press: 1;
}

/* ============================================================
 * 组件：房间设置弹窗选项按钮（选项卡玻璃化）
 * ============================================================ */
html.liquid-glass .sheet-body .opt.selected {
  --lg-edge: 9px;
  --lg-blur: 6px;
  --lg-spec-r: 64px;
  --lg-bright: 1.03;
  --lg-tint:
    linear-gradient(
      155deg,
      color-mix(in srgb, var(--md-primary) 42%, rgb(255 255 255 / 0.24)),
      color-mix(in srgb, var(--md-primary) 58%, rgb(255 255 255 / 0.06)) 55%,
      color-mix(in srgb, var(--md-primary) 36%, rgb(255 255 255 / 0.2))
    );
  --lg-tint-edge:
    linear-gradient(135deg, rgb(255 255 255 / 0.18), rgb(255 255 255 / 0.04));
  border-color: color-mix(in srgb, var(--md-primary) 55%, rgb(255 255 255 / 0.3));
  box-shadow:
    0 1px 2px rgb(0 0 0 / 0.14),
    0 3px 12px rgb(0 0 0 / 0.16);
  color: var(--md-on-primary);
}
html.liquid-glass .sheet-body .opt.selected:active {
  transform: scale(0.96);
}

/* ---- 减少动态效果偏好：停用呼吸形变与高光过渡 ---- */
@media (prefers-reduced-motion: reduce) {
  html.liquid-glass .lg-glass,
  html.liquid-glass .lg-glass::before,
  html.liquid-glass .lg-glass::after {
    transition: none;
    animation: none;
  }
}
  }

/* ---- 成员卡专属色调：可见的半透明表面（不可用 transparent，否则卡片隐形） ---- */
html.liquid-glass .list-user-card,
html.liquid-glass .all-card,
html.liquid-glass .mini-card {
  --lg-tint:
    linear-gradient(
      160deg,
      color-mix(in srgb, var(--md-surface-container-low) 28%, transparent),
      color-mix(in srgb, var(--md-surface-container-low) 42%, transparent) 55%,
      color-mix(in srgb, var(--md-surface-container-low) 24%, transparent)
    );
  --lg-tint-edge:
    linear-gradient(to right, rgb(255 72 72 / 0.14), transparent 36%),
    linear-gradient(to left, rgb(72 138 255 / 0.14), transparent 36%);
}

/* ---- 字体衬底 chip 玻璃化 ----
 * 浮动标签/下拉标签骑在输入框 1px 边框线上，chip 的存在意义是遮住那段
 * 边框 —— 不能全透（边框会从文字中间穿过），降为半透明衬底：
 * 自定义背景图透过来不再是一块实色补丁。 ---- */
html.liquid-glass .md-text-field.float-label .tf-label.floated {
  background: color-mix(in srgb, var(--md-surface) 60%, transparent);
}
html.liquid-glass .md-dropdown .dd-label.dd-label {
  background: color-mix(in srgb, var(--md-surface) 60%, transparent);
}
html.liquid-glass .rf-label {
  background: color-mix(in srgb, var(--md-surface-container-low) 60%, transparent);
}
/* 快捷网络配置行背景（30% surface 补丁）→ 全透，仅留 1px 分组线 */
html.liquid-glass .switch-row {
  background: transparent;
}
/* 拓扑连线延迟标签（88% surface 补丁）→ 45% 透 */
html.liquid-glass .edge-label {
  background: color-mix(in srgb, var(--md-surface) 45%, transparent);
}
</style>

