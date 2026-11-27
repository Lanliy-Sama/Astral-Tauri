<script>
  /** Bottom sheet aggregating every member's advertised Minecraft LAN worlds. */
  import Icon from '$lib/components/common/Icon.svelte';
  import GrassBlockIcon from './cards/GrassBlockIcon.svelte';
  import { roomSocial } from '$lib/services/RoomSocial.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { writeClipboard } from '$lib/rust/Api.svelte';
  import { snackSuccess } from '$lib/components/common/SnackbarStore.svelte';

  let { open = $bindable(false) } = $props();

  const games = $derived.by(() => {
    if (!sm.appSettingsState.experimentalFeatures) return [];
    const own = roomSocial.ownLanGames.map((game) => ({
      ...game, owner: sm.playerState.playerName || '本机玩家',
    }));
    const remote = roomSocial.members.flatMap((member) => (member.lanGames || []).map((game) => ({
      ...game, owner: member.nickname || '房员',
    })));
    return [...own, ...remote].sort((a, b) => a.owner.localeCompare(b.owner) || a.port - b.port);
  });

  async function copyAddress(game) {
    const address = `${game.ip}:${game.port}`;
    await writeClipboard(address);
    snackSuccess('已复制局域网地址', address, { duration: 2000 });
  }
</script>

<svelte:window onkeydown={(event) => {
  if (open && event.key === 'Escape') open = false;
}} />

{#if open}
  <div class="sheet-backdrop" role="presentation" onclick={() => (open = false)}>
    <section
      class="lan-sheet"
      role="dialog"
      aria-modal="true"
      aria-label="Minecraft 局域网游戏列表"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
    >
      <div class="sheet-handle"></div>
      <header class="sheet-head">
        <div class="sheet-title-icon"><GrassBlockIcon size={38} /></div>
        <div class="sheet-title-copy">
          <h2>Minecraft 局域网游戏</h2>
          <p>{games.length ? `${games.length} 个可加入的局域网世界` : '自动发现房间成员开放的世界'}</p>
        </div>
        <button class="md-icon-btn" aria-label="关闭局域网游戏列表" onclick={() => (open = false)}>
          <Icon name="close" />
        </button>
      </header>

      <div class="game-list">
        {#each games as game (game.owner + game.ip + ':' + game.port)}
          <article class="game-card">
            <div class="game-cover">
              {#if game.cover}
                <img src={game.cover} alt={`${game.owner} 的服务器封面`} />
              {:else}
                <GrassBlockIcon size={48} />
              {/if}
            </div>
            <div class="game-info">
              <h3>{game.owner} 的局域网游戏</h3>
              <p class="motd" data-tip={game.motd}>{game.motd}</p>
              <div class="address-row">
                <span class="address">{game.ip}:{game.port}</span>
                <button class="md-btn md-tonal-btn copy-address" onclick={() => copyAddress(game)}>
                  <Icon name="copy" size={17} />复制地址
                </button>
              </div>
            </div>
          </article>
        {:else}
          <div class="empty-state">
            <GrassBlockIcon size={62} />
            <strong>暂未发现局域网世界</strong>
            <span>在 Minecraft 单人世界中选择“对局域网开放”，游戏会自动出现在这里。</span>
          </div>
        {/each}
      </div>
    </section>
  </div>
{/if}

<style>
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1200;
    background: rgb(0 0 0 / 0.34);
    backdrop-filter: blur(3px);
    animation: backdrop-in 180ms ease;
  }
  .lan-sheet {
    position: absolute;
    left: 50%;
    bottom: 0;
    width: min(720px, 100%);
    max-height: min(76vh, 760px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--md-outline-variant);
    border-bottom: 0;
    border-radius: 26px 26px 0 0;
    background: color-mix(in srgb, var(--md-surface) 94%, transparent);
    box-shadow: 0 -18px 54px rgb(0 0 0 / 0.28);
    transform: translateX(-50%);
    animation: sheet-up 300ms cubic-bezier(0.05, 0.7, 0.1, 1);
  }
  .sheet-handle { width: 40px; height: 4px; flex-shrink: 0; margin: 9px auto 2px; border-radius: 4px; background: var(--md-outline-variant); }
  .sheet-head { display: flex; align-items: center; gap: 13px; padding: 14px 18px 13px; border-bottom: 1px solid var(--md-outline-variant); }
  .sheet-title-icon { flex-shrink: 0; }
  .sheet-title-copy { flex: 1; min-width: 0; }
  .sheet-title-copy h2 { margin: 0; font-size: 20px; line-height: 27px; }
  .sheet-title-copy p { margin: 2px 0 0; color: var(--md-on-surface-variant); font-size: 11px; }
  .game-list { min-height: 0; overflow-y: auto; padding: 14px 18px calc(20px + env(safe-area-inset-bottom)); }
  .game-card {
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 13px;
    border: 1px solid var(--md-outline-variant);
    border-radius: 16px;
    background: color-mix(in srgb, var(--md-surface-container-low) 78%, transparent);
  }
  .game-card + .game-card { margin-top: 11px; }
  .game-cover {
    width: 76px;
    height: 76px;
    flex: 0 0 76px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border: 2px solid rgb(255 255 255 / 0.2);
    border-radius: 7px;
    background: linear-gradient(#629a3f 0 28%, #745035 28% 100%);
    image-rendering: pixelated;
  }
  .game-cover img { width: 100%; height: 100%; object-fit: cover; image-rendering: pixelated; }
  .game-info { flex: 1; min-width: 0; }
  .game-info h3 { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 16px; line-height: 23px; }
  .motd { margin: 3px 0 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--md-on-surface-variant); font-size: 12px; }
  .address-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .address { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--md-primary); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 12px; }
  .copy-address { min-height: 32px; display: inline-flex; align-items: center; gap: 5px; padding: 0 10px; flex-shrink: 0; font-size: 11px; }
  .empty-state { min-height: 260px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 28px; color: var(--md-on-surface-variant); text-align: center; }
  .empty-state strong { color: var(--md-on-surface); font-size: 16px; }
  .empty-state span { max-width: 390px; font-size: 12px; line-height: 20px; }
  @keyframes backdrop-in { from { opacity: 0; } to { opacity: 1; } }
  @keyframes sheet-up { from { transform: translate(-50%, 100%); } to { transform: translate(-50%, 0); } }
  @media (max-width: 520px) {
    .game-card { gap: 11px; padding: 11px; }
    .game-cover { width: 62px; height: 62px; flex-basis: 62px; }
    .address-row { align-items: flex-start; flex-direction: column; }
  }
  @media (prefers-reduced-motion: reduce) {
    .sheet-backdrop, .lan-sheet { animation: none; }
  }
</style>
