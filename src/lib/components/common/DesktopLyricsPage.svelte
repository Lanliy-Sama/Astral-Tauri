<script>
  /** Frameless always-on-top desktop lyrics window, driven by the main player. */
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';

  let state = $state({ title: 'Astral', playing: false, line: null });

  function auxiliary(line, key) {
    if (!line || /[\u3400-\u9fff]/.test(line.l || '')) return '';
    return line[key] || '';
  }

  async function startDrag(event) {
    if (event.button !== 0 || event.target.closest('button')) return;
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().startDragging();
    } catch { /* 鼠标穿透开启时不响应拖动 */ }
  }

  async function closeWindow() {
    const [{ emit }, { getCurrentWebviewWindow }] = await Promise.all([
      import('@tauri-apps/api/event'),
      import('@tauri-apps/api/webviewWindow'),
    ]);
    await emit('desktop-lyrics-close-request');
    await getCurrentWebviewWindow().close();
  }

  onMount(() => {
    let unlisten = null;
    import('@tauri-apps/api/event')
      .then(({ listen }) => listen('desktop-lyrics-update', (event) => {
        state = { title: 'Astral', playing: false, line: null, ...event.payload };
      }))
      .then((dispose) => (unlisten = dispose))
      .catch(() => {});
    return () => unlisten?.();
  });
</script>

<svelte:head>
  <style>
    html, body, #app { margin: 0; background: transparent !important; overflow: hidden; }
  </style>
</svelte:head>

<main class="desktop-lyrics" data-tauri-drag-region onpointerdown={startDrag}>
  <div class="lyrics-content" class:playing={state.playing} data-tauri-drag-region aria-live="polite">
    <div class="original" data-tauri-drag-region>{state.line?.l || state.title}</div>
    {#if auxiliary(state.line, 'tr')}
      <div class="auxiliary" data-tauri-drag-region>{auxiliary(state.line, 'tr')}</div>
    {/if}
    {#if auxiliary(state.line, 'ro')}
      <div class="auxiliary romanized" data-tauri-drag-region>{auxiliary(state.line, 'ro')}</div>
    {/if}
  </div>
  <button class="close-button" aria-label="关闭桌面歌词" onclick={closeWindow}>
    <Icon name="close" size={18} color="rgba(255,255,255,.72)" />
  </button>
</main>

<style>
  .desktop-lyrics {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px 10px 24px;
    border: 1px solid rgb(255 255 255 / 0.14);
    border-radius: 22px;
    background: rgb(15 18 27 / 0.94);
    box-shadow: 0 12px 36px rgb(0 0 0 / 0.26), inset 0 1px 0 rgb(255 255 255 / 0.1);
    backdrop-filter: blur(18px) saturate(1.2);
    color: #fff;
    cursor: move;
    user-select: none;
  }
  .lyrics-content { flex: 1; min-width: 0; text-align: center; text-shadow: 0 2px 9px rgb(0 0 0 / 0.45); }
  .lyrics-content.playing .original { color: #fff; }
  .original { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 22px; line-height: 31px; font-weight: 700; letter-spacing: .02em; }
  .auxiliary { margin-top: 1px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: rgb(255 255 255 / 0.72); font-size: 12px; line-height: 17px; }
  .romanized { color: rgb(255 255 255 / 0.52); font-size: 11px; }
  .close-button { width: 32px; height: 32px; flex: 0 0 32px; display: grid; place-items: center; border: 0; border-radius: 50%; background: transparent; cursor: pointer; }
  .close-button:hover { background: rgb(255 255 255 / 0.12); }
  @media (max-width: 560px) {
    .original { font-size: 17px; line-height: 25px; }
    .auxiliary { font-size: 10px; line-height: 14px; }
  }
</style>
