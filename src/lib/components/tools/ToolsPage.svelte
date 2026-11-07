<script>
  /**
   * 工具页：联机相关工具入口枢纽 —— 对应 lib/features/tools/pages/tools_page.dart。
   * CustomScrollView + SliverPadding(16)；魔法墙卡仅 Windows（浏览器预览模式也显示以便验证）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import MagicWallPage from '$lib/components/magicwall/MagicWallPage.svelte';
  import PortWhitelistPage from './PortWhitelistPage.svelte';
  import NatTestPage from '$lib/components/nat/NatTestPage.svelte';
  import { pushPage } from '$lib/router/PageRouter.svelte';
  import { isWindows } from '$lib/utils/Platform.svelte';

  /* 魔法墙仅 Windows 提供（WFP 规则） */
  const showMagicWall = isWindows();

  function go(target) {
    return () => pushPage(target);
  }

  function onCardKeydown(e, target) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      pushPage(target);
    }
  }
</script>

<div class="tools-scroll">
  <div class="tools-inner">
    <!-- _buildSectionTitle：Padding(4,0,0,8) + 12 -->
    <div
      class="md-title-medium"
      style="padding:0 0 8px 4px; margin:0 0 12px 0; font-weight:600; color:var(--md-on-surface)"
    >联机工具</div>

    {#if showMagicWall}
      <div
        class="md-card tool-card"
        role="button"
        tabindex="0"
        onclick={go(MagicWallPage)}
        onkeydown={(e) => onCardKeydown(e, MagicWallPage)}
      >
        <div class="md-list-tile two-line tool-tile">
          <span class="tile-leading">
            <Icon name="security" size={24} color="var(--md-primary)" />
          </span>
          <span class="tile-content">
            <span class="tile-title" style="font-weight:500">魔法墙</span>
            <span class="tile-subtitle" style="font-size:12px; line-height:16px">高级防火墙管理</span>
          </span>
          <span class="tile-trailing">
            <Icon name="chevron_right" color="var(--md-on-surface-variant)" />
          </span>
        </div>
      </div>
    {/if}

    <div
      class="md-card tool-card"
      role="button"
      tabindex="0"
      onclick={go(PortWhitelistPage)}
      onkeydown={(e) => onCardKeydown(e, PortWhitelistPage)}
    >
      <div class="md-list-tile two-line tool-tile">
        <span class="tile-leading">
          <!-- Icons.security_outlined：Material Symbols 无该连字名，用基名字形（FILL 0 描边盾牌） -->
          <span class="ms" style="font-size:24px; color:var(--md-primary)">security</span>
        </span>
        <span class="tile-content">
          <span class="tile-title" style="font-weight:500">端口白名单</span>
          <span class="tile-subtitle" style="font-size:12px; line-height:16px"
            >配置TCP/UDP端口访问白名单</span
          >
        </span>
        <span class="tile-trailing">
          <Icon name="chevron_right" color="var(--md-on-surface-variant)" />
        </span>
      </div>
    </div>

    <div
      class="md-card tool-card"
      role="button"
      tabindex="0"
      onclick={go(NatTestPage)}
      onkeydown={(e) => onCardKeydown(e, NatTestPage)}
    >
      <div class="md-list-tile two-line tool-tile">
        <span class="tile-leading">
          <Icon name="network_check" size={24} color="var(--md-primary)" />
        </span>
        <span class="tile-content">
          <span class="tile-title" style="font-weight:500">NAT 类型测试</span>
          <span class="tile-subtitle" style="font-size:12px; line-height:16px"
            >检测您的网络 NAT 类型</span
          >
        </span>
        <span class="tile-trailing">
          <Icon name="chevron_right" color="var(--md-on-surface-variant)" />
        </span>
      </div>
    </div>

    <div class="end-spacer"></div>
  </div>
</div>

<style>
  .tools-scroll {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }
  /* SliverPadding(16) */
  .tools-inner {
    padding: 16px;
  }
  /* ListTile contentPadding h20 v12 */
  .tools-inner .md-list-tile {
    padding: 12px 20px;
  }
  /* 卡片间距 8 */
  .tool-card + .tool-card {
    margin-top: 8px;
  }
  /* 末尾 SizedBox(32) */
  .end-spacer {
    height: 32px;
  }
</style>
