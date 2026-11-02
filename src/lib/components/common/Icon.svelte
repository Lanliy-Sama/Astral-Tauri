<script>
  /**
   * Material 图标组件。
   * Flutter Material Icons → Material Symbols 名称映射；
   * filled 属性对应经典 Material Icons 的实心字形（FILL 轴）。
   */
  let { name, size = 24, filled = false, color = 'inherit', style = '' } = $props();

  // Flutter 图标名 → Material Symbols 连字名（仅列出有差异的）
  const NAME_MAP = {
    nightlight_round: 'nightlight',
    power_settings_new_rounded: 'power_settings_new',
    cloud_off_rounded: 'cloud_off',
    people_outline: 'people',
    error_outline: 'error',
    info_outline: 'info',
    home_outlined: 'home',
    room_preferences_outlined: 'room_preferences',
    build_outlined: 'build',
    dns_outlined: 'dns',
    settings_outlined: 'settings',
    article_outlined: 'article',
    timer_outlined: 'timer',
    upload_rounded: 'upload',
    download_rounded: 'download',
    arrow_upward_rounded: 'arrow_upward',
    arrow_downward_rounded: 'arrow_downward',
    link_rounded: 'link',
    close_rounded: 'close',
    warning_amber_outlined: 'warning',
    chevron_right: 'chevron_right',
    list_alt: 'list_alt',
    network_check: 'network_check',
    bar_chart_outlined: 'bar_chart',
    layers_outlined: 'layers',
    tune_outlined: 'tune',
    share: 'share',
    paste: 'content_paste',
    copy_outlined: 'content_copy',
    copy_all: 'content_copy',
    clear_all: 'delete_sweep',
    device_unknown: 'device_unknown',
    check_circle: 'check_circle',
    memory: 'memory',
  };

  const symbol = $derived(NAME_MAP[name] || name);
  // 经典 Material Icons 中这些图标本身即实心字形
  const FILLED_BY_DEFAULT = new Set([
    'person', 'add', 'delete', 'edit', 'close', 'check', 'block', 'circle',
    'public', 'security', 'meeting_room', 'search', 'lock', 'lock_open',
    'share', 'sort', 'paste', 'copy', 'route', 'lan', 'refresh', 'dns',
    'home', 'room_preferences', 'build', 'settings', 'language', 'remove',
    'menu', 'check_circle', 'error', 'info', 'wifi', 'lan_outlined',
    'workspace_premium', 'military_tech', 'verified', 'android', 'apple',
    'speed', 'hub', 'sync_alt', 'trending_up', 'trending_down', 'router',
    'label', 'layers', 'playlist_add', 'swap_horiz', 'computer', 'cloud',
    'smartphone', 'power_settings_new', 'link', 'window', 'terminal',
    'phone_iphone', 'devices', 'minimize', 'play_arrow', 'tag', 'list',
    'notifications', 'update', 'history', 'science', 'auto_awesome',
    'launch', 'bolt', 'sync', 'system_update', 'install_mobile',
    'settings_input_antenna', 'shield', 'network_wifi', 'vpn_lock',
    'storage', 'keyboard_arrow_down', 'article', 'arrow_upward',
    'arrow_downward', 'upload', 'download', 'timer', 'format_quote',
    'chevron_right', 'expand_more', 'expand_less', 'settings_ethernet',
    'cloud_download', 'device_unknown', 'memory',
    'colorize', 'autorenew', 'account_tree', 'bar_chart', 'list_alt', 'network_check',
    'color_lens', 'auto_mode',
  ]);

</script>

<span
  class="ms {filled || FILLED_BY_DEFAULT.has(name) ? 'filled' : ''}"
  style="font-size:{size}px; color:{color}; {style}"
>{symbol}</span>
