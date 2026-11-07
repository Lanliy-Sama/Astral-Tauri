<script module>
/** 枚举常量（Svelte 5 模块）。 */
/** 全局枚举 —— 与 Dart 端一一对应 */

/** CoState: 连接状态机 */
export const CoState = Object.freeze({
  idle: 'idle',
  connecting: 'connecting',
  connected: 'connected',
});

/** MainTab: 主导航页索引（与 MainTab 枚举序一致） */
export const MainTab = Object.freeze({
  home: 0,
  room: 1,
  tools: 2,
  servers: 3,
  settings: 4,
});

/** ThemeMode */
export const ThemeMode = Object.freeze({
  system: 'system',
  light: 'light',
  dark: 'dark',
});

/** UserSortOption: 用户排序 */
export const UserSortOption = Object.freeze({
  none: 0,
  latency: 1,
  nameLength: 2,
});

/** UserSortOrder */
export const UserSortOrder = Object.freeze({
  ascending: 0,
  descending: 1,
});

/** UserDisplayMode */
export const UserDisplayMode = Object.freeze({
  all: 0,
  users: 1,
  servers: 2,
});

/** UserListStyle */
export const UserListStyle = Object.freeze({
  simple: 0,
  detailed: 1,
  list: 2,
});

/** ServerStatus: 服务器在线状态 */
export const ServerStatus = Object.freeze({
  online: 'online',
  offline: 'offline',
  inUse: 'inUse',
  unknown: 'unknown',
});
</script>
