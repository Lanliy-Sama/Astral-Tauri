<script>
  /** Minecraft LAN server entries styled after the vanilla multiplayer list. */
  import Icon from '$lib/components/common/Icon.svelte';
  import GrassBlockIcon from './GrassBlockIcon.svelte';
  import { roomSocial } from '$lib/services/RoomSocial.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { writeClipboard } from '$lib/rust/Api.svelte';
  import { snackSuccess } from '$lib/components/common/SnackbarStore.svelte';

  let { player, playerName = '', compact = false } = $props();

  const games = $derived.by(() => {
    if (!sm.appSettingsState.experimentalFeatures) return [];
    if (player.ipv4 && player.ipv4 === roomSocial.localIp) return roomSocial.ownLanGames;
    return roomSocial.members.find((member) => member.peer === player.ipv4)?.lanGames || [];
  });

  async function copyAddress(game) {
    const address = `${game.ip}:${game.port}`;
    await writeClipboard(address);
    snackSuccess('已复制局域网地址', address, { duration: 2000 });
  }
</script>

{#if games.length}
  <div class="lan-section" class:compact role="group" aria-label="Minecraft 局域网房间" onclick={(event) => event.stopPropagation()} onkeydown={(event) => event.stopPropagation()}>
    <div class="lan-heading">
      <Icon name="sports_esports" size={17} color="var(--md-primary)" />
      <span>Minecraft 局域网房间</span>
    </div>
    {#each games as game (game.ip + ':' + game.port)}
      <div class="lan-server">
        <div class="server-cover">
          {#if game.cover}
            <img src={game.cover} alt="服务器封面" loading="lazy" />
          {:else}
            <Icon name="sports_esports" size={compact ? 26 : 38} color="rgba(255,255,255,.9)" />
          {/if}
        </div>
        <div class="server-copy">
          <div class="server-name">{playerName || '玩家'} 的局域网游戏</div>
          <div class="server-motd" data-tip={game.motd}>{game.motd}</div>
          <button class="server-address" data-tip="点击复制 IP 和端口" onclick={() => copyAddress(game)}>
            <span>{game.ip}:{game.port}</span>
            <Icon name="copy" size={15} />
          </button>
        </div>
      </div>
    {/each}
    <button class="lan-list-button" onclick={() => window.dispatchEvent(new CustomEvent('astral-open-lan-sheet'))}>
      <GrassBlockIcon size={20} />
      <span>Minecraft 局域网列表</span>
      <Icon name="chevron_right" size={17} />
    </button>
  </div>
{/if}

<style>
  .lan-section {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid color-mix(in srgb, var(--md-outline-variant) 60%, transparent);
  }
  .lan-heading {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
    color: var(--md-on-surface-variant);
    font-size: 11px;
    font-weight: 700;
  }
  .lan-server {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 7px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--md-surface-container) 62%, transparent);
  }
  .lan-server + .lan-server { margin-top: 7px; }
  .server-cover {
    width: 64px;
    height: 64px;
    flex: 0 0 64px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border: 2px solid rgb(255 255 255 / 0.18);
    border-radius: 5px;
    background: linear-gradient(#558b35 0 28%, #6d4b2c 28% 100%);
    image-rendering: pixelated;
  }
  .server-cover img { width: 100%; height: 100%; object-fit: cover; image-rendering: pixelated; }
  .server-copy { flex: 1; min-width: 0; }
  .server-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-on-surface);
    font-size: 14px;
    line-height: 20px;
    font-weight: 700;
  }
  .server-motd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-on-surface-variant);
    font-size: 11px;
    line-height: 17px;
  }
  .server-address {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 100%;
    margin-top: 3px;
    padding: 2px 6px;
    border: 0;
    border-radius: 6px;
    background: var(--md-primary-06);
    color: var(--md-primary);
    cursor: pointer;
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 10px;
  }
  .server-address span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .server-address:hover { background: var(--md-primary-08); }
  .lan-list-button {
    width: 100%;
    min-height: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    margin-top: 9px;
    border: 1px solid color-mix(in srgb, #69a844 42%, var(--md-outline-variant));
    border-radius: 10px;
    background: color-mix(in srgb, #69a844 10%, transparent);
    color: var(--md-on-surface);
    cursor: pointer;
    font-size: 12px;
    font-weight: 700;
  }
  .lan-list-button:hover { background: color-mix(in srgb, #69a844 17%, transparent); }
  .compact .server-cover { width: 44px; height: 44px; flex-basis: 44px; }
  .compact .server-name { font-size: 12px; line-height: 16px; }
  .compact .server-motd { font-size: 10px; line-height: 14px; }
</style>
