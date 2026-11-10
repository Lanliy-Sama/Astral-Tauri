<script>
  /**
   * 灵动岛音量胶囊 —— Ctrl+↑/↓ 调整背景音乐音量时，主屏顶部居中弹出，
   * 显示音量图标 + 进度条 + 数值，1.5s 后自动收起。
   * 显示触发器：sm.bgmState.volumePulse 自增（BgmPlayer.bgmAdjustVolume）。
   */
  import { sm } from '../../services/ServiceManager.svelte';

  let visible = $state(false);
  let timer = 0;

  $effect(() => {
    void sm.bgmState.volumePulse;
    if (!sm.bgmState.volumePulse) return;
    visible = true;
    clearTimeout(timer);
    timer = setTimeout(() => (visible = false), 1500);
    return () => clearTimeout(timer);
  });
</script>

<div class="bgm-capsule" class:show={visible} aria-hidden={!visible}>
  <span class="ms filled cap-icon">
    {sm.bgmState.volumeDelta >= 0 ? 'volume_up' : 'volume_down'}
  </span>
  <div class="cap-bar">
    <div class="cap-fill" style="width:{sm.bgmState.volume}%"></div>
  </div>
  <span class="cap-num">{sm.bgmState.volume}</span>
</div>

<style>
  .bgm-capsule {
    position: fixed;
    top: 14px;
    left: 50%;
    transform: translate(-50%, -16px);
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 16px;
    border-radius: 999px;
    background: rgb(24 24 27 / 0.92);
    color: #ffffff;
    box-shadow:
      0 4px 14px rgb(0 0 0 / 0.28),
      inset 0 0 0 1px rgb(255 255 255 / 0.08);
    backdrop-filter: blur(12px) saturate(1.4);
    z-index: 3500; /* 高于 snackbar(2000)/tooltip(3000) */
    pointer-events: none;
    opacity: 0;
    transition:
      opacity 220ms ease,
      transform 300ms cubic-bezier(0.05, 0.7, 0.1, 1);
  }
  .bgm-capsule.show {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  .cap-icon {
    font-size: 20px;
    width: 20px;
    height: 20px;
  }
  .cap-bar {
    width: 120px;
    height: 5px;
    border-radius: 3px;
    background: rgb(255 255 255 / 0.22);
    overflow: hidden;
  }
  .cap-fill {
    height: 100%;
    border-radius: 3px;
    background: linear-gradient(to right, rgb(255 255 255 / 0.85), #ffffff);
    transition: width 140ms ease;
  }
  .cap-num {
    font-size: 13px;
    font-weight: 700;
    line-height: 16px;
    min-width: 26px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
