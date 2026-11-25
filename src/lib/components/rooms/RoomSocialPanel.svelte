<script>
  /** 房间协作侧栏：聊天室 / 多频道语音 / 只读一起看。 */
  import { onMount } from 'svelte';
  import Icon from '../common/Icon.svelte';
  import Switch from '../common/Switch.svelte';
  import { roomSocial } from '../../services/RoomSocial.svelte';
  import { voiceService } from '../../services/VoiceService.svelte';
  import { watchService } from '../../services/WatchService.svelte';
  import { desktopService } from '../../services/RemoteDesktopService.svelte';
  import { fileShare } from '../../services/FileShare.svelte';

  let { open = $bindable(false) } = $props();
  let tab = $state('chat');
  let chatText = $state('');
  let chatError = $state('');
  let chatScroll = null;
  let newChannelName = $state('');
  let newChannelHidden = $state(false);
  let newChannelInviteOnly = $state(false);
  let inviteTargets = $state({});
  let watchUrl = $state('https://www.bilibili.com');
  let videoEl = null;
  let desktopVideoEl = null;
  let fileInput = null;

  const videoSites = [
    { name: '抖音', url: 'https://www.douyin.com' },
    { name: 'Bilibili', url: 'https://www.bilibili.com' },
    { name: '快手', url: 'https://www.kuaishou.com' },
  ];

  onMount(() => voiceService.start());

  $effect(() => {
    void roomSocial.messages.length;
    if (tab === 'chat' && chatScroll) {
      requestAnimationFrame(() => chatScroll?.scrollTo({ top: chatScroll.scrollHeight, behavior: 'smooth' }));
    }
  });

  $effect(() => {
    const url = watchService.remoteUrl;
    if (!videoEl) return;
    videoEl.src = url || '';
    if (url) videoEl.play().catch(() => {});
  });

  $effect(() => {
    const url = desktopService.remoteUrl;
    if (!desktopVideoEl) return;
    desktopVideoEl.src = url || '';
    if (url) desktopVideoEl.play().catch(() => {});
  });

  function desktopPointer(event, kind = 'move') {
    const rect = event.currentTarget.getBoundingClientRect();
    const x = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
    const y = Math.min(1, Math.max(0, (event.clientY - rect.top) / rect.height));
    const button = event.button === 2 ? 'right' : event.button === 1 ? 'middle' : 'left';
    desktopService.sendPointer(kind, x, y, button);
  }

  function desktopKey(event, down) {
    event.preventDefault();
    desktopService.sendKey(down, event.keyCode);
  }

  function formatBytes(size) {
    const value = Number(size) || 0;
    if (value >= 1024 ** 3) return `${(value / 1024 ** 3).toFixed(2)} GB`;
    if (value >= 1024 ** 2) return `${(value / 1024 ** 2).toFixed(1)} MB`;
    if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
    return `${value} B`;
  }

  function timeText(timestamp) {
    return new Date(timestamp).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  }

  async function sendChat() {
    chatError = '';
    try {
      await roomSocial.sendChat(chatText);
      chatText = '';
    } catch (e) {
      chatError = String(e?.message || e);
    }
  }

  async function createChannel() {
    try {
      await voiceService.createChannel(newChannelName, {
        hidden: newChannelHidden, inviteOnly: newChannelInviteOnly,
      });
      newChannelName = '';
    } catch { /* voiceService.error 已显示 */ }
  }

  async function joinChannel(channel) {
    try { await voiceService.join(channel); } catch { /* 面板显示错误 */ }
  }

  async function startWatch() {
    try { await watchService.requestWatch(watchUrl); } catch { /* 面板显示错误 */ }
  }
</script>

<aside class="social-panel" class:open aria-hidden={!open}>
  <div class="panel-head">
    <div class="head-copy">
      <div class="panel-title">房间协作</div>
      <div class="panel-subtitle">{roomSocial.peers.length} 位房员可通信 · {roomSocial.accounts.length} 个共享音乐账号</div>
    </div>
    <button class="md-icon-btn" aria-label="关闭房间协作" onclick={() => (open = false)}>
      <Icon name="close" />
    </button>
  </div>

  <div class="social-tabs" role="tablist">
    <button class:selected={tab === 'chat'} onclick={() => (tab = 'chat')} role="tab">
      <Icon name="chat_bubble" size={19} />聊天室
    </button>
    <button class:selected={tab === 'voice'} onclick={() => (tab = 'voice')} role="tab">
      <Icon name="podcasts" size={19} />语音
      {#if voiceService.invites.length}<span class="tab-badge">{voiceService.invites.length}</span>{/if}
    </button>
    <button class:selected={tab === 'watch'} onclick={() => (tab = 'watch')} role="tab">
      <Icon name="live_tv" size={19} />一起看
    </button>
    <button class:selected={tab === 'files'} onclick={() => (tab = 'files')} role="tab">
      <Icon name="folder_shared" size={19} />文件
      {#if fileShare.files.length}<span class="tab-badge">{fileShare.files.length}</span>{/if}
    </button>
  </div>

  <div class="panel-body">
    {#if tab === 'chat'}
      <div class="chat-layout">
        <div class="messages" bind:this={chatScroll}>
          {#if !roomSocial.messages.length}
            <div class="empty-state">
              <Icon name="forum" size={44} color="var(--md-outline)" />
              <span>还没有消息，和房间里的玩家打个招呼吧</span>
            </div>
          {/if}
          {#each roomSocial.messages as message (message.id)}
            <div class="message" class:local={message.local}>
              <div class="message-meta">{message.local ? '我' : message.sender} · {timeText(message.sentAt)}</div>
              <div class="message-bubble">{message.text}</div>
            </div>
          {/each}
        </div>
        {#if chatError}<div class="error-text">{chatError}</div>{/if}
        <div class="chat-compose">
          <textarea
            bind:value={chatText}
            maxlength="2000"
            placeholder="输入房间消息…"
            aria-label="聊天消息"
            onkeydown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                sendChat();
              }
            }}
          ></textarea>
          <button class="md-fab small" aria-label="发送消息" disabled={!chatText.trim()} onclick={sendChat}>
            <Icon name="send" size={20} />
          </button>
        </div>
      </div>
    {:else if tab === 'voice'}
      <div class="voice-layout">
        <section class="social-card create-card">
          <div class="section-title"><Icon name="add" size={19} />创建语音频道</div>
          <div class="voice-nickname">当前房间昵称：<strong>{sm.playerState.playerName || '房员'}</strong></div>
          <input class="social-input" bind:value={newChannelName} maxlength="32" placeholder="频道名称" />
          <div class="create-options">
            <label><Switch bind:value={newChannelHidden} />隐藏频道</label>
            <label><Switch bind:value={newChannelInviteOnly} />仅限邀请</label>
          </div>
          <button class="md-btn md-tonal-btn full-btn" disabled={!newChannelName.trim() || voiceService.busy} onclick={createChannel}>
            创建并加入
          </button>
        </section>

        {#if voiceService.activeChannelId}
          <section class="social-card active-voice">
            <div>
              <div class="section-title">正在语音</div>
              <div class="active-name">{voiceService.channels.find((c) => c.id === voiceService.activeChannelId)?.name}</div>
            </div>
            <button class="md-icon-btn" data-tip={voiceService.muted ? '取消静音' : '静音'} onclick={() => voiceService.toggleMute()}>
              <Icon name={voiceService.muted ? 'mic_off' : 'mic'} color={voiceService.muted ? 'var(--md-error)' : 'var(--md-primary)'} />
            </button>
            <button class="md-btn md-text-btn" onclick={() => voiceService.leave()}>离开</button>
          </section>
        {/if}
        {#if voiceService.error}<div class="error-text">{voiceService.error}</div>{/if}

        <div class="channel-list">
          {#each voiceService.channels as channel (channel.id)}
            <section class="social-card channel-card" class:active={voiceService.activeChannelId === channel.id}>
              <div class="channel-head">
                <div class="channel-title-wrap">
                  <Icon name={channel.hidden ? 'visibility_off' : channel.inviteOnly ? 'lock' : 'podcasts'} size={20} />
                  <div>
                    <div class="channel-name">{channel.name}</div>
                    <div class="channel-owner">频道主：{channel.ownerName}</div>
                  </div>
                </div>
                <button
                  class="md-btn {voiceService.activeChannelId === channel.id ? 'md-text-btn' : 'md-tonal-btn'} compact-btn"
                  disabled={voiceService.busy}
                  onclick={() => voiceService.activeChannelId === channel.id ? voiceService.leave() : joinChannel(channel)}
                >
                  {voiceService.activeChannelId === channel.id ? '离开' : '加入'}
                </button>
              </div>
              <div class="channel-badges">
                {#if channel.hidden}<span>隐藏</span>{/if}
                {#if channel.inviteOnly}<span>邀请制</span>{/if}
                <span>{channel.members.length} 人</span>
              </div>

              <div class="member-list">
                {#each channel.members as member (member.peer)}
                  <div class="member-row">
                    <span class="member-name">{member.nickname}{member.peer === channel.owner ? '（频道主）' : ''}</span>
                    {#if member.muted}<Icon name="mic_off" size={17} color="var(--md-error)" />{/if}
                    {#if member.peer !== roomSocial.localIp}
                      <button class="md-icon-btn compact" data-tip={`请求控制 ${member.nickname} 的桌面`} onclick={() => desktopService.request(member)}>
                        <Icon name="desktop_windows" size={18} color="var(--md-primary)" />
                      </button>
                    {/if}
                    {#if voiceService.isOwner(channel) && member.peer !== roomSocial.localIp}
                      <button class="md-icon-btn compact" data-tip={member.muted ? '解除禁言' : '禁言'} onclick={() => voiceService.setMemberMuted(channel.id, member.peer, !member.muted)}>
                        <Icon name={member.muted ? 'mic' : 'mic_off'} size={18} />
                      </button>
                      <button class="md-icon-btn compact danger" data-tip="踢出频道" onclick={() => voiceService.kickMember(channel.id, member.peer)}>
                        <Icon name="person_remove" size={18} color="var(--md-error)" />
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>

              {#if voiceService.isOwner(channel)}
                <div class="owner-tools">
                  <div class="owner-title">频道主管理</div>
                  <label class="policy-row">
                    <span>隐藏频道（仅邀请可见）</span>
                    <Switch value={channel.hidden} onChange={(value) => voiceService.setPolicy(channel.id, { hidden: value })} />
                  </label>
                  <label class="policy-row">
                    <span>需要邀请才可加入</span>
                    <Switch value={channel.inviteOnly} onChange={(value) => voiceService.setPolicy(channel.id, { inviteOnly: value })} />
                  </label>
                  <div class="invite-row">
                    <select
                      class="social-input"
                      aria-label="选择要邀请的房员"
                      value={inviteTargets[channel.id] || ''}
                      onchange={(e) => (inviteTargets[channel.id] = e.currentTarget.value)}
                    >
                      <option value="">选择房员</option>
                      {#each roomSocial.members.filter((m) => !channel.members.some((member) => member.peer === m.peer)) as member (member.peer)}
                        <option value={member.peer}>{member.nickname}</option>
                      {/each}
                    </select>
                    <button
                      class="md-btn md-outlined-btn compact-btn"
                      disabled={!inviteTargets[channel.id]}
                      onclick={() => voiceService.invite(channel.id, inviteTargets[channel.id])}
                    >邀请</button>
                  </div>
                </div>
              {/if}
            </section>
          {:else}
            <div class="empty-state"><Icon name="podcasts" size={42} color="var(--md-outline)" /><span>暂无公开语音频道</span></div>
          {/each}
        </div>

        <section class="social-card desktop-section">
          <div class="section-title"><Icon name="desktop_windows" size={20} />频道内远程桌面</div>
          {#if desktopService.incoming}
            <div class="desktop-request">
              <strong>{desktopService.incoming.requester} 请求控制你的桌面</strong>
              <p>同意后需要你手动选择采集画面，对方将能查看并控制鼠标和键盘。你可以随时结束。</p>
              <div class="request-actions">
                <button class="md-btn md-filled-btn" disabled={desktopService.starting} onclick={() => desktopService.accept()}>
                  {desktopService.starting ? '等待采集授权…' : '手动同意并选择画面'}
                </button>
                <button class="md-btn md-text-btn" onclick={() => desktopService.decline()}>拒绝</button>
              </div>
            </div>
          {:else if desktopService.outgoing}
            <div class="desktop-pending">正在等待 {desktopService.outgoing.nickname} 手动同意…</div>
            <button class="md-btn md-text-btn" onclick={() => desktopService.cancelRequest()}>取消请求</button>
          {:else if desktopService.session}
            <div class="desktop-session-head">
              <div>
                <strong>{desktopService.hosting ? `${desktopService.session.controller} 正在控制此设备` : `正在控制 ${desktopService.session.host}`}</strong>
                <div class="desktop-resolution">固定采集分辨率：{desktopService.resolution}</div>
              </div>
              <button class="md-btn md-text-btn" onclick={() => desktopService.stop()}>结束</button>
            </div>

            <div class="bitrate-control lg-glass lg-spec">
              <div class="bitrate-label"><span>视频码率</span><span>原生分辨率不缩放</span></div>
              <div class="bitrate-row">
                <input
                  class="bitrate-slider"
                  type="range"
                  min="1"
                  max="500"
                  step="1"
                  value={desktopService.bitrate}
                  style={`--bitrate-progress:${((desktopService.bitrate - 1) / 499) * 100}%`}
                  oninput={(e) => desktopService.setBitrate(Number(e.currentTarget.value))}
                  aria-label="远程桌面码率，单位 Mbps"
                />
                <div class="bitrate-number">
                  <input
                    type="number"
                    min="1"
                    max="500"
                    step="1"
                    value={desktopService.bitrate}
                    onclick={(e) => e.currentTarget.select()}
                    onchange={(e) => desktopService.setBitrate(Number(e.currentTarget.value))}
                    aria-label="直接输入码率"
                  />
                  <span>Mbps</span>
                </div>
              </div>
            </div>

            <div class="frame-rate-control">
              <div class="bitrate-label">
                <span>最高帧率</span>
                <span>{desktopService.actualFrameRate ? `实际采集 ${desktopService.actualFrameRate} FPS` : '由采集设备能力决定'}</span>
              </div>
              <div class="frame-rate-options">
                {#each [30, 60, 120, 144, 165, 240] as rate (rate)}
                  <button
                    class:selected={desktopService.frameRate === rate}
                    onclick={() => desktopService.setFrameRate(rate)}
                    aria-pressed={desktopService.frameRate === rate}
                  >{rate}</button>
                {/each}
              </div>
            </div>

            {#if desktopService.joined && !desktopService.hosting}
              <div
                class="desktop-stage"
                tabindex="0"
                aria-label="远程桌面控制画面，单击后发送鼠标键盘输入"
                onpointermove={(e) => desktopPointer(e)}
                onpointerdown={(e) => {
                  e.currentTarget.focus();
                  e.currentTarget.setPointerCapture(e.pointerId);
                  desktopPointer(e, 'down');
                }}
                onpointerup={(e) => desktopPointer(e, 'up')}
                onwheel={(e) => {
                  e.preventDefault();
                  desktopService.sendWheel(e.deltaY);
                }}
                oncontextmenu={(e) => e.preventDefault()}
                onkeydown={(e) => desktopKey(e, true)}
                onkeyup={(e) => desktopKey(e, false)}
              >
                <video bind:this={desktopVideoEl} autoplay playsinline tabindex="-1"></video>
                {#if !desktopService.remoteUrl}<div class="video-loading">正在接收被控端原生画面…</div>{/if}
              </div>
              <div class="desktop-control-hint">画面已获得控制权限：移动/点击鼠标、滚轮和键盘事件将发送给被控端</div>
            {:else}
              <div class="desktop-control-hint">你正在共享并允许对方控制。结束会话会立即停止画面与全部输入权限。</div>
            {/if}
          {:else}
            <p class="desktop-help">点击频道成员旁的桌面图标发起请求。只有对方手动同意后才会建立一对一控制连接。</p>
          {/if}
          {#if desktopService.error}<div class="error-text">{desktopService.error}</div>{/if}
        </section>
      </div>
    {:else if tab === 'files'}
      <div class="files-layout">
        <section class="social-card file-share-head">
          <div>
            <div class="section-title"><Icon name="folder_shared" size={20} />房间文件分享</div>
            <p>选择文件后仅广播元数据，成员点击下载时才从分享端传输文件。</p>
          </div>
          <button class="md-btn md-tonal-btn" disabled={fileShare.sharing} onclick={() => fileInput?.click()}>
            {fileShare.sharing ? '正在准备…' : '选择文件'}
          </button>
          <input type="file" multiple hidden bind:this={fileInput} onchange={(event) => {
            fileShare.shareFiles(event.currentTarget.files);
            event.currentTarget.value = '';
          }} />
        </section>
        {#if fileShare.error}<div class="error-text">{fileShare.error}</div>{/if}
        <div class="file-list">
          {#each fileShare.files as file (file.ownerPeer + file.id)}
            <section class="social-card file-row">
              <div class="file-icon"><Icon name="insert_drive_file" size={26} color="var(--md-primary)" /></div>
              <div class="file-copy">
                <div class="file-name" data-tip={file.name}>{file.name}</div>
                <div class="file-meta">{formatBytes(file.size)} · {file.ownerName}</div>
                {#if fileShare.downloads[file.id]?.path}
                  <div class="file-path selectable">{fileShare.downloads[file.id].path}</div>
                {/if}
                {#if fileShare.downloads[file.id]?.error}
                  <div class="error-text">{fileShare.downloads[file.id].error}</div>
                {/if}
              </div>
              {#if file.ownerPeer === roomSocial.localIp}
                <span class="file-own">已分享</span>
              {:else}
                <button class="md-btn md-outlined-btn compact-btn" disabled={fileShare.downloads[file.id]?.status === 'downloading'} onclick={() => fileShare.download(file)}>
                  {fileShare.downloads[file.id]?.status === 'downloading' ? '下载中…' : fileShare.downloads[file.id]?.status === 'done' ? '再次下载' : '下载'}
                </button>
              {/if}
            </section>
          {:else}
            <div class="empty-state"><Icon name="folder_open" size={44} color="var(--md-outline)" /><span>暂无文件分享，选择本地文件开始分享</span></div>
          {/each}
        </div>
      </div>
    {:else}
      <div class="watch-layout">
        {#if watchService.incoming}
          <section class="social-card watch-request">
            <Icon name="live_tv" size={30} color="var(--md-primary)" />
            <div class="watch-request-copy">
              <strong>{watchService.incoming.session.host} 发起一起看</strong>
              <span>{watchService.incoming.session.platform} · 仅接收画面，不可交互</span>
            </div>
            <div class="request-actions">
              <button class="md-btn md-filled-btn" onclick={() => watchService.accept()}>同意加入</button>
              <button class="md-btn md-text-btn" onclick={() => watchService.decline()}>拒绝</button>
            </div>
          </section>
        {/if}

        {#if watchService.joined && !watchService.hosting}
          <section class="watch-stage">
            <video bind:this={videoEl} autoplay playsinline tabindex="-1" aria-label="共享主机实时画面"></video>
            {#if !watchService.remoteUrl}
              <div class="video-loading"><span class="ms">cast_connected</span>正在接收共享主机画面…</div>
            {/if}
            <div class="watch-status">由 {watchService.session?.host} 控制 · 你只能观看，无法操作画面或视频进度</div>
            <button class="md-btn md-text-btn" onclick={() => watchService.stop()}>退出一起看</button>
          </section>
        {:else if watchService.hosting}
          <section class="social-card hosting-card">
            <Icon name="cast" size={38} color="var(--md-primary)" />
            <div class="hosting-title">你正在共享 {watchService.session?.platform}</div>
            <p>请在系统采集选择器中选择刚打开的网站窗口或标签页。只有同意加入的房员能收到画面。</p>
            <div class="guest-count">{watchService.guests.length} 位房员正在观看</div>
            {#each watchService.guests as guest (guest.peer)}<div class="guest-row">{guest.nickname}</div>{/each}
            <button class="md-btn md-filled-btn" onclick={() => watchService.stop()}>结束共享</button>
          </section>
        {:else}
          <section class="social-card">
            <div class="section-title"><Icon name="live_tv" size={20} />发起一起看</div>
            <div class="site-tabs">
              {#each videoSites as site (site.url)}
                <button class:selected={watchUrl === site.url} onclick={() => (watchUrl = site.url)}>{site.name}</button>
              {/each}
            </div>
            <input class="social-input" bind:value={watchUrl} placeholder="https://…" aria-label="视频网站地址" />
            <p class="watch-note">你将作为共享主机。应用打开 HTTPS 网站后，由你明确选择要采集的窗口或标签页；房员同意后才会收到实时只读画面。</p>
            <button class="md-btn md-tonal-btn full-btn" disabled={watchService.starting || !watchUrl.startsWith('https://')} onclick={startWatch}>
              {watchService.starting ? '等待画面采集授权…' : '申请发起一起看'}
            </button>
          </section>
        {/if}
        {#if watchService.error}<div class="error-text">{watchService.error}</div>{/if}
        <div class="watch-disclaimer">不转发网站登录态、Cookie、DOM 或输入事件。DRM 视频和浏览器保护内容可能无法采集；同步精度取决于 TCP 媒体通道的网络状况。</div>
      </div>
    {/if}
  </div>
</aside>

<style>
  .social-panel {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(410px, 100%);
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--md-surface) 94%, transparent);
    border-left: 1px solid var(--md-outline-variant);
    box-shadow: -12px 0 36px rgb(0 0 0 / 0.18);
    backdrop-filter: blur(22px) saturate(1.2);
    z-index: 80;
    transform: translateX(104%);
    opacity: 0;
    pointer-events: none;
    transition: transform 300ms cubic-bezier(0.05, 0.7, 0.1, 1), opacity 200ms ease;
  }
  .social-panel.open { transform: translateX(0); opacity: 1; pointer-events: auto; }
  .panel-head { display: flex; align-items: center; gap: 12px; padding: 14px 12px 10px 18px; }
  .head-copy { flex: 1; min-width: 0; }
  .panel-title { font-size: 19px; line-height: 26px; font-weight: 700; }
  .panel-subtitle { font-size: 11px; color: var(--md-on-surface-variant); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .social-tabs { display: grid; grid-template-columns: repeat(4, 1fr); gap: 5px; padding: 0 12px 10px; }
  .social-tabs button {
    position: relative;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border: 0;
    border-radius: 14px;
    background: transparent;
    color: var(--md-on-surface-variant);
    cursor: pointer;
  }
  .social-tabs button.selected { background: var(--md-secondary-container); color: var(--md-on-secondary-container); }
  .tab-badge { min-width: 17px; height: 17px; padding: 0 4px; display: grid; place-items: center; border-radius: 9px; background: var(--md-error); color: var(--md-on-error); font-size: 10px; }
  .panel-body { flex: 1; min-height: 0; overflow: hidden; }
  .chat-layout, .voice-layout, .watch-layout, .files-layout { height: 100%; }
  .files-layout { overflow-y: auto; padding: 2px 12px 20px; }
  .file-share-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .file-share-head p { margin: 0; color: var(--md-on-surface-variant); font-size: 10px; line-height: 16px; }
  .file-share-head .md-btn { flex-shrink: 0; }
  .file-row { display: flex; align-items: center; gap: 10px; }
  .file-icon { flex-shrink: 0; }
  .file-copy { flex: 1; min-width: 0; }
  .file-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; font-weight: 600; }
  .file-meta { margin-top: 3px; color: var(--md-on-surface-variant); font-size: 10px; }
  .file-own { color: var(--md-primary); font-size: 10px; white-space: nowrap; }
  .file-path { margin-top: 7px; color: var(--md-primary); font-size: 9px; line-height: 14px; word-break: break-all; user-select: text; }
  @media (max-width: 480px) {
    .file-share-head { align-items: flex-start; flex-direction: column; }
  }
  .chat-layout { display: flex; flex-direction: column; }
  .messages { flex: 1; min-height: 0; overflow-y: auto; padding: 8px 16px 14px; }
  .message { display: flex; flex-direction: column; align-items: flex-start; margin-bottom: 13px; animation: message-in 180ms ease; }
  .message.local { align-items: flex-end; }
  .message-meta { margin: 0 6px 4px; color: var(--md-on-surface-variant); font-size: 10px; }
  .message-bubble { max-width: 88%; padding: 9px 12px; border-radius: 6px 16px 16px 16px; background: var(--md-surface-container-high); line-height: 20px; user-select: text; white-space: pre-wrap; word-break: break-word; }
  .message.local .message-bubble { border-radius: 16px 6px 16px 16px; background: var(--md-primary); color: var(--md-on-primary); }
  .chat-compose { display: flex; align-items: flex-end; gap: 10px; padding: 10px 14px calc(14px + env(safe-area-inset-bottom)); border-top: 1px solid var(--md-outline-variant); }
  .chat-compose textarea { flex: 1; min-width: 0; height: 54px; max-height: 110px; resize: vertical; padding: 10px 12px; border: 1px solid var(--md-outline-variant); border-radius: 15px; background: var(--md-surface-container-low); outline: none; user-select: text; }
  .chat-compose textarea:focus { border-color: var(--md-primary); }
  .voice-layout, .watch-layout { overflow-y: auto; padding: 2px 12px 20px; }
  .social-card { padding: 14px; margin-bottom: 10px; border: 1px solid var(--md-outline-variant); border-radius: 18px; background: color-mix(in srgb, var(--md-surface-container-low) 88%, transparent); }
  .section-title { display: flex; align-items: center; gap: 7px; margin-bottom: 11px; font-size: 14px; font-weight: 700; }
  .social-input { width: 100%; min-height: 40px; margin-bottom: 9px; padding: 8px 11px; border: 1px solid var(--md-outline-variant); border-radius: 12px; background: var(--md-surface); color: var(--md-on-surface); outline: none; user-select: text; }
  .social-input:focus { border-color: var(--md-primary); }
  .voice-nickname { margin-bottom: 10px; padding: 9px 11px; border-radius: 12px; background: var(--md-primary-06); color: var(--md-on-surface-variant); font-size: 12px; }
  .voice-nickname strong { color: var(--md-primary); }
  .create-options { display: flex; flex-wrap: wrap; gap: 14px; margin: 4px 0 12px; }
  .create-options label { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--md-on-surface-variant); }
  .full-btn { width: 100%; }
  .active-voice { display: flex; align-items: center; gap: 8px; border-color: var(--md-primary); }
  .active-voice > div:first-child { flex: 1; min-width: 0; }
  .active-voice .section-title { margin: 0; font-size: 11px; color: var(--md-on-surface-variant); }
  .active-name { font-size: 15px; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .channel-card.active { border-color: var(--md-primary); }
  .channel-head { display: flex; align-items: center; gap: 8px; }
  .channel-title-wrap { flex: 1; min-width: 0; display: flex; align-items: center; gap: 9px; }
  .channel-name { font-size: 15px; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .channel-owner { font-size: 10px; color: var(--md-on-surface-variant); }
  .compact-btn { height: 34px; min-height: 34px; padding: 0 13px; }
  .channel-badges { display: flex; gap: 5px; margin: 9px 0; }
  .channel-badges span { padding: 2px 7px; border-radius: 7px; background: var(--md-primary-06); color: var(--md-primary); font-size: 10px; }
  .member-row { min-height: 32px; display: flex; align-items: center; gap: 5px; border-top: 1px solid color-mix(in srgb, var(--md-outline-variant) 55%, transparent); }
  .member-name { flex: 1; min-width: 0; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .danger { color: var(--md-error); }
  .owner-tools { margin-top: 9px; padding-top: 9px; border-top: 1px solid var(--md-outline-variant); }
  .owner-title { margin-bottom: 5px; color: var(--md-primary); font-size: 11px; font-weight: 700; }
  .policy-row { min-height: 40px; display: flex; align-items: center; justify-content: space-between; gap: 12px; font-size: 11px; }
  .policy-row :global(.md-switch) { transform: scale(0.78); transform-origin: right center; }
  .invite-row { display: grid; grid-template-columns: 1fr auto; gap: 8px; align-items: start; margin-top: 6px; }
  .invite-row .social-input { margin: 0; }
  .site-tabs { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; margin-bottom: 12px; }
  .site-tabs button { height: 34px; border: 1px solid var(--md-outline-variant); border-radius: 11px; background: transparent; color: var(--md-on-surface); cursor: pointer; }
  .site-tabs button.selected { background: var(--md-secondary-container); border-color: transparent; }
  .desktop-section { margin-top: 14px; }
  .desktop-request strong, .desktop-session-head strong { font-size: 13px; line-height: 19px; }
  .desktop-request p, .desktop-help { margin: 8px 0 12px; color: var(--md-on-surface-variant); font-size: 11px; line-height: 18px; }
  .desktop-pending { padding: 8px 0; color: var(--md-primary); font-size: 12px; }
  .desktop-session-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; margin-bottom: 12px; }
  .desktop-resolution { margin-top: 4px; color: var(--md-on-surface-variant); font-size: 10px; }
  .bitrate-control { padding: 12px; border-radius: 16px; background: var(--md-primary-06); }
  .bitrate-label { display: flex; justify-content: space-between; gap: 8px; margin-bottom: 12px; font-size: 10px; color: var(--md-on-surface-variant); }
  .bitrate-row { display: grid; grid-template-columns: minmax(0, 1fr) 104px; align-items: center; gap: 12px; }
  .bitrate-slider { width: 100%; height: 24px; margin: 0; appearance: none; background: transparent; cursor: pointer; }
  .bitrate-slider::-webkit-slider-runnable-track {
    height: 8px;
    border-radius: 999px;
    background: linear-gradient(to right, var(--md-primary) 0 var(--bitrate-progress), color-mix(in srgb, var(--md-primary) 18%, transparent) var(--bitrate-progress) 100%);
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.18), 0 1px 0 rgb(255 255 255 / 0.35);
  }
  .bitrate-slider::-webkit-slider-thumb {
    width: 22px;
    height: 22px;
    margin-top: -7px;
    appearance: none;
    border: 1px solid rgb(255 255 255 / 0.7);
    border-radius: 50%;
    background: radial-gradient(circle at 32% 25%, #fff, color-mix(in srgb, var(--md-primary) 45%, #fff) 48%, var(--md-primary));
    box-shadow: 0 2px 7px rgb(0 0 0 / 0.25), inset 0 1px 2px #fff;
    transition: transform 140ms ease;
  }
  .bitrate-slider:active::-webkit-slider-thumb { transform: scale(1.18, 0.9); }
  .bitrate-number { height: 38px; display: flex; align-items: center; gap: 4px; padding: 0 8px; border: 1px solid var(--md-outline-variant); border-radius: 12px; background: var(--md-surface); }
  .bitrate-number input { width: 52px; border: 0; outline: 0; background: transparent; text-align: right; font-variant-numeric: tabular-nums; user-select: text; }
  .bitrate-number span { color: var(--md-on-surface-variant); font-size: 10px; }
  .frame-rate-control { margin-top: 12px; }
  .frame-rate-options { display: grid; grid-template-columns: repeat(6, 1fr); gap: 5px; }
  .frame-rate-options button { height: 31px; padding: 0; border: 1px solid var(--md-outline-variant); border-radius: 10px; background: transparent; color: var(--md-on-surface-variant); cursor: pointer; font-size: 10px; transition: background 150ms ease, transform 150ms ease; }
  .frame-rate-options button.selected { border-color: transparent; background: var(--md-primary); color: var(--md-on-primary); }
  .frame-rate-options button:active { transform: scale(0.93); }
  .desktop-stage { position: relative; margin-top: 12px; overflow: hidden; border-radius: 14px; background: #050507; outline: none; cursor: crosshair; touch-action: none; }
  .desktop-stage:focus { box-shadow: 0 0 0 2px var(--md-primary); }
  .desktop-stage video { width: 100%; display: block; pointer-events: none; user-select: none; }
  .desktop-control-hint { padding: 9px 2px 0; color: var(--md-on-surface-variant); font-size: 10px; line-height: 16px; }
  .watch-note, .hosting-card p { margin: 4px 0 13px; color: var(--md-on-surface-variant); font-size: 11px; line-height: 18px; }
  .watch-request { text-align: center; }
  .watch-request-copy { display: flex; flex-direction: column; gap: 5px; margin: 10px 0; }
  .watch-request-copy span { color: var(--md-on-surface-variant); font-size: 11px; }
  .request-actions { display: flex; justify-content: center; gap: 6px; }
  .watch-stage { position: relative; min-height: 300px; display: flex; flex-direction: column; align-items: center; }
  .watch-stage video { width: 100%; aspect-ratio: 16 / 9; border-radius: 18px; background: #000; object-fit: contain; pointer-events: none; user-select: none; }
  .video-loading { position: absolute; top: 0; left: 0; right: 0; aspect-ratio: 16 / 9; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; border-radius: 18px; background: #101013; color: #fff; pointer-events: none; }
  .video-loading .ms { font-size: 36px; }
  .watch-status { padding: 10px 4px; color: var(--md-on-surface-variant); font-size: 11px; text-align: center; }
  .hosting-card { text-align: center; }
  .hosting-title { margin: 10px 0; font-size: 17px; font-weight: 700; }
  .guest-count { margin: 12px 0 6px; color: var(--md-primary); }
  .guest-row { padding: 5px; font-size: 12px; }
  .hosting-card .md-btn { margin-top: 14px; }
  .watch-disclaimer { padding: 4px 8px 16px; color: var(--md-on-surface-variant); font-size: 10px; line-height: 17px; }
  .empty-state { min-height: 150px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 24px; color: var(--md-on-surface-variant); font-size: 12px; text-align: center; }
  .error-text { margin: 6px 4px 10px; padding: 9px 11px; border-radius: 11px; background: var(--md-error-container); color: var(--md-on-error-container); font-size: 11px; line-height: 17px; }
  @keyframes message-in { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: translateY(0); } }
  @media (max-width: 480px) {
    .social-panel { width: 100%; border-left: 0; }
    .social-tabs button { font-size: 12px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .social-panel, .message { transition: none; animation: none; }
  }
</style>
