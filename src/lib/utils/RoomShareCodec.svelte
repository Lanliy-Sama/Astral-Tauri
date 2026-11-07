<script module>
/** 房间/网络配置分享码编解码（Svelte 5 模块）。 */
/**
 * 房间分享编解码 —— 翻译自 lib/core/room/room_share_codec.dart。
 * shareCode = base64Url(gzip(utf8(json)))，去除 '=' 填充；
 * 链接格式 astral://room?code=<shareCode>。
 */

const APP_SCHEME = 'astral';
const ROOM_PATH = 'room';

/* ---------- gzip / gunzip（原生 CompressionStream） ---------- */
async function gzip(bytes) {
  const cs = new CompressionStream('gzip');
  const stream = new Blob([bytes]).stream().pipeThrough(cs);
  return new Uint8Array(await new Response(stream).arrayBuffer());
}
async function gunzip(bytes) {
  const ds = new DecompressionStream('gzip');
  const stream = new Blob([bytes]).stream().pipeThrough(ds);
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

function bytesToBase64Url(bytes) {
  let binary = '';
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}
function base64UrlToBytes(str) {
  let s = str.replace(/-/g, '+').replace(/_/g, '/');
  while (s.length % 4 !== 0) s += '=';
  const binary = atob(s);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

/* ---------- Room <-> 短键 JSON ---------- */
function roomToJson(room, includeNetworkConfig) {
  const obj = {
    n: room.name || '',
    r: room.roomName || '',
    p: room.password || '',
    m: room.messageKey || '',
    e: room.encrypted ? 1 : 0,
  };
  if (room.servers && room.servers.length > 0) obj.s = room.servers;
  if (room.customParam && room.customParam.length > 0) obj.c = room.customParam;
  if (
    includeNetworkConfig &&
    room.networkConfigJson &&
    room.networkConfigJson.length > 0
  ) {
    try {
      obj.net = JSON.parse(room.networkConfigJson);
    } catch {
      /* 解析失败则不带配置 */
    }
  }
  return obj;
}

function jsonToRoom(obj) {
  if (!obj || typeof obj !== 'object') return null;
  const room = {
    id: null,
    name: typeof obj.n === 'string' ? obj.n : '',
    roomName: typeof obj.r === 'string' ? obj.r : '',
    password: typeof obj.p === 'string' ? obj.p : '',
    messageKey: typeof obj.m === 'string' ? obj.m : '',
    encrypted: obj.e === 1,
    tags: [],
    servers: Array.isArray(obj.s) ? obj.s : [],
    customParam: typeof obj.c === 'string' ? obj.c : '',
    networkConfigJson: obj.net != null ? JSON.stringify(obj.net) : '',
    sortOrder: 0,
  };
  return room;
}

/* ---------- 校验 / 清洗（validateRoom / cleanRoom） ---------- */
const ILLEGAL_CHARS = /[<>:"/\\|?*]/;

export function validateRoom(room) {
  if (room == null) return [false, '房间对象为空'];
  if (!room.name || !room.name.trim()) return [false, '房间名称不能为空'];
  if (room.name.length > 50) return [false, '房间名称过长，不能超过50个字符'];
  if (ILLEGAL_CHARS.test(room.name)) return [false, '房间名称包含非法字符'];
  if (!room.encrypted) {
    if (!room.roomName || !room.roomName.trim())
      return [false, '公开房间必须有房间号'];
    if (room.roomName.length > 100)
      return [false, '房间号过长，不能超过100个字符'];
    if (room.password && room.password.length > 100)
      return [false, '房间密码过长，不能超过100个字符'];
  }
  if (room.tags && room.tags.length > 10) return [false, '标签数量不能超过10个'];
  for (const tag of room.tags || []) {
    if (tag.length > 20) return [false, '标签长度不能超过20个字符'];
    if (ILLEGAL_CHARS.test(tag)) return [false, '标签包含非法字符'];
  }
  return [true, null];
}

export function cleanRoom(room) {
  const trim = (v) => (typeof v === 'string' ? v.trim() : v);
  return {
    ...room,
    name: trim(room.name),
    roomName: trim(room.roomName),
    password: trim(room.password),
    messageKey: trim(room.messageKey),
    customParam: trim(room.customParam),
    tags: (room.tags || []).map((t) => t.trim()).filter((t) => t.length > 0),
  };
}

/* ---------- 编解码 ---------- */
export async function encryptRoom(room, { includeNetworkConfig = false } = {}) {
  try {
    const json = JSON.stringify(roomToJson(room, includeNetworkConfig));
    const compressed = await gzip(new TextEncoder().encode(json));
    return bytesToBase64Url(compressed);
  } catch {
    return null;
  }
}

export async function decryptRoom(shareCode) {
  try {
    const bytes = base64UrlToBytes(shareCode);
    const decompressed = await gunzip(bytes);
    const json = new TextDecoder().decode(decompressed);
    return jsonToRoom(JSON.parse(json));
  } catch {
    return null;
  }
}

export function generateShareLink(shareCode) {
  return `${APP_SCHEME}://${ROOM_PATH}?code=${shareCode}`;
}

export function generateShareText(room, link, { hasServers = false, hasNetworkConfig = false } = {}) {
  const lines = [];
  lines.push('🎮 Astral 房间分享');
  lines.push('');
  lines.push(`🏠 房间：${room.name}`);
  lines.push(room.encrypted ? '🔒 加密房间' : '🔓 公开房间');
  if (room.tags && room.tags.length > 0) lines.push(`🏷️ ${room.tags.join('、')}`);
  lines.push('');
  lines.push('📦 分享选项：');
  if (hasServers) lines.push('✓ 携带服务器列表');
  if (hasNetworkConfig) lines.push('✓ 携带网络配置');
  lines.push('');
  lines.push(`🔗 分享链接：${link}`);
  lines.push('');
  lines.push('使用说明：');
  lines.push('1. 复制上面的分享链接');
  lines.push('2. 在 Astral 中点击"导入房间"');
  lines.push('3. 粘贴链接并确认导入');
  lines.push('');
  lines.push('⏰ 分享链接有效期：30天');
  return lines.join('\n');
}

/** 从任意文本/链接提取分享码 */
export function extractShareCode(text) {
  const cleaned = (text || '').replace(/\s+/g, '');
  if (!cleaned) return null;
  if (cleaned.startsWith(`${APP_SCHEME}://`)) {
    try {
      const url = new URL(cleaned);
      return url.searchParams.get('code');
    } catch {
      return null;
    }
  }
  return cleaned;
}

/* ---------- NetworkConfigShare（房间携带的网络配置） ---------- */
export function networkConfigShareFromState(state) {
  return {
    dhcp: state.dhcp === true ? 1 : 0,
    proto: state.defaultProtocol || null,
    enc: state.enableEncryption === true ? 1 : 0,
    lat: state.latencyFirst === true ? 1 : 0,
    dp2p: state.disableP2p === true ? 1 : 0,
    dudp: state.disableUdpHolePunching === true ? 1 : 0,
    dtcp: state.disableTcpHolePunching === true ? 1 : 0,
    dsym: state.disableSymHolePunching === true ? 1 : 0,
    comp: state.dataCompressAlgo ?? null,
    kcp: state.enableKcpProxy === true ? 1 : 0,
    bind: state.bindDevice === true ? 1 : 0,
    tun: state.noTun === true ? 1 : 0,
  };
}

export function parseNetworkConfigShare(jsonString) {
  try {
    const o = JSON.parse(jsonString);
    if (!o || typeof o !== 'object') return null;
    return {
      dhcp: o.dhcp == null ? true : o.dhcp === 1,
      defaultProtocol: o.proto ?? null,
      enableEncryption: o.enc == null ? null : o.enc === 1,
      latencyFirst: o.lat == null ? null : o.lat === 1,
      disableP2p: o.dp2p == null ? null : o.dp2p === 1,
      disableUdpHolePunching: o.dudp == null ? null : o.dudp === 1,
      disableTcpHolePunching: o.dtcp == null ? null : o.dtcp === 1,
      disableSymHolePunching: o.dsym == null ? null : o.dsym === 1,
      dataCompressAlgo: o.comp ?? null,
      enableKcpProxy: o.kcp == null ? null : o.kcp === 1,
      bindDevice: o.bind == null ? null : o.bind === 1,
      noTun: o.tun == null ? null : o.tun === 1,
    };
  } catch {
    return null;
  }
}

/** 可读摘要（导入确认对话框展示） */
export function networkConfigShareSummary(share) {
  const lines = [];
  const yn = (b) => (b ? '开启' : '关闭');
  lines.push(`• DHCP: ${share.dhcp ? '自动' : '手动'}`);
  if (share.defaultProtocol) lines.push(`• 默认协议: ${share.defaultProtocol.toUpperCase()}`);
  if (share.enableEncryption != null) lines.push(`• 加密: ${yn(share.enableEncryption)}`);
  if (share.latencyFirst != null) lines.push(`• 延迟优先: ${yn(share.latencyFirst)}`);
  if (share.disableP2p != null) lines.push(`• P2P: ${share.disableP2p ? '禁用' : '启用'}`);
  if (share.disableUdpHolePunching != null)
    lines.push(`• UDP打洞: ${share.disableUdpHolePunching ? '禁用' : '启用'}`);
  if (share.disableTcpHolePunching != null)
    lines.push(`• TCP打洞: ${share.disableTcpHolePunching ? '禁用' : '启用'}`);
  if (share.disableSymHolePunching != null)
    lines.push(`• 对称打洞: ${share.disableSymHolePunching ? '禁用' : '启用'}`);
  if (share.dataCompressAlgo != null)
    lines.push(`• 压缩算法: ${share.dataCompressAlgo === 1 ? '无压缩' : '高性能压缩'}`);
  if (share.enableKcpProxy != null) lines.push(`• KCP代理: ${yn(share.enableKcpProxy)}`);
  if (share.bindDevice != null) lines.push(`• 绑定设备: ${yn(share.bindDevice)}`);
  if (share.noTun != null) lines.push(`• TUN设备: ${share.noTun ? '禁用' : '启用'}`);
  return lines;
}
</script>
