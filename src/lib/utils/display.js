/**
 * 展示工具集 —— 平台版本解析 / 连接样式 / NAT 显示 / 流量格式化 / 随机房间名。
 * 全部翻译自 lib/shared/utils 与 features/rooms/widgets 中的对应 Dart 逻辑。
 */

/* ---------- PlatformVersionParser ---------- */
const PLATFORM_MAP = {
  windows: { name: 'Windows', icon: 'window' },
  linux: { name: 'Linux', icon: 'terminal' },
  android: { name: 'Android', icon: 'android' },
  macos: { name: 'macOS', icon: 'apple' },
  mac: { name: 'macOS', icon: 'apple' },
  ios: { name: 'iOS', icon: 'phone_iphone' },
};

/** 版本串 "版本号|平台名" 解析 */
export function parsePlatformVersion(version) {
  if (!version || !version.includes('|')) return { number: '', platform: '', name: '', icon: 'memory' };
  const [number, platformRaw] = version.split('|');
  const platform = (platformRaw || '').trim();
  const mapped = PLATFORM_MAP[platform.toLowerCase()];
  if (mapped) return { number: number.trim(), platform, name: mapped.name, icon: mapped.icon };
  return { number: number.trim(), platform, name: platform, icon: 'devices' };
}

/* ---------- PeerConnectionStyle ---------- */

/** 连接类型判定（mapConnectionType） */
export function mapConnectionType(cost, ip, localIp) {
  if (ip === '0.0.0.0') return '服务器';
  if (localIp && ip === localIp) return '本机';
  if (cost === 1) return '直链';
  if (cost >= 2) return '中转';
  return '未知';
}

/** 连接类型颜色（CSS 变量或硬编码色） */
export function connectionTypeColor(type) {
  if (type.includes('server') || type === '服务器') return 'var(--flutter-deep-purple)';
  if (type.includes('p2p') || type === '直链') return 'var(--flutter-green)';
  if (type.includes('relay') || type === '中转') return 'var(--flutter-orange)';
  if (type.includes('direct') || type === '本机') return 'var(--md-primary)';
  return 'var(--flutter-grey)';
}

/** 连接类型图标 */
export function connectionTypeIcon(type) {
  if (type.includes('server') || type === '服务器') return 'dns';
  if (type.includes('p2p') || type === '直链') return 'link';
  if (type.includes('relay') || type === '中转') return 'swap_horiz';
  if (type.includes('direct') || type === '本机') return 'computer';
  return 'device_unknown';
}

/** 延迟颜色：<50 绿 / <100 橙 / 其余红 */
export function latencyColor(ms) {
  if (ms < 50) return 'var(--flutter-green)';
  if (ms < 100) return 'var(--flutter-orange)';
  return 'var(--flutter-red)';
}

/** 丢包颜色：<1 绿 / <5 橙 / 其余红 */
export function lossColor(loss) {
  if (loss < 1.0) return 'var(--flutter-green)';
  if (loss < 5.0) return 'var(--flutter-orange)';
  return 'var(--flutter-red)';
}

/** 隧道协议格式化：tcp→tcp4，udp→udp4 */
export function formatTunnelProto(proto) {
  if (!proto) return '';
  return proto
    .split(',')
    .map((p) => p.trim())
    .map((p) => (p === 'tcp' ? 'tcp4' : p === 'udp' ? 'udp4' : p))
    .join(',');
}

/* ---------- NAT 显示（两套体系） ---------- */

/** MiniUserCardNat：游戏化等级（WoW 品质色） */
export const NAT_MINI_MAP = {
  Unknown: { label: '未知', icon: 'help_outline', color: 'var(--flutter-grey)' },
  OpenInternet: { label: '传奇', icon: 'workspace_premium', color: 'var(--nat-legendary)' },
  NoPat: { label: '传奇', icon: 'workspace_premium', color: 'var(--nat-legendary)' },
  FullCone: { label: '史诗', icon: 'military_tech', color: 'var(--nat-epic)' },
  Restricted: { label: '优质', icon: 'verified', color: 'var(--nat-rare)' },
  PortRestricted: { label: '优质', icon: 'verified', color: 'var(--nat-rare)' },
  Symmetric: { label: '困难', icon: 'block', color: 'var(--nat-difficult)' },
  SymUdpFirewall: { label: '普通', icon: 'circle', color: 'var(--nat-common)' },
  SymmetricEasyInc: { label: '普通', icon: 'circle', color: 'var(--nat-common)' },
  SymmetricEasyDec: { label: '普通', icon: 'circle', color: 'var(--nat-common)' },
};
export function natMiniInfo(nat) {
  return NAT_MINI_MAP[nat] || NAT_MINI_MAP.Unknown;
}

/** AllUserCardNat：技术文案 */
export const NAT_FULL_MAP = {
  Unknown: { label: '未知', icon: 'help_outline', color: 'var(--flutter-grey)' },
  OpenInternet: { label: '开放网络', icon: 'public', color: 'var(--flutter-green)' },
  NoPat: { label: '无PAT', icon: 'router', color: 'var(--flutter-green)' },
  FullCone: { label: '全锥形', icon: 'public', color: 'var(--flutter-green)' },
  Restricted: { label: '受限锥形', icon: 'shield', color: 'var(--flutter-orange)' },
  PortRestricted: { label: '端口受限锥形', icon: 'security', color: 'var(--flutter-orange)' },
  Symmetric: { label: '对称型', icon: 'sync_alt', color: 'var(--flutter-red)' },
  SymUdpFirewall: { label: '对称UDP防火墙', icon: 'fireplace', color: 'var(--flutter-red)' },
  SymmetricEasyInc: { label: '对称递增型', icon: 'trending_up', color: 'var(--flutter-red)' },
  SymmetricEasyDec: { label: '对称递减型', icon: 'trending_down', color: 'var(--flutter-red)' },
};
export function natFullInfo(nat) {
  return NAT_FULL_MAP[nat] || NAT_FULL_MAP.Unknown;
}

/* ---------- 流量格式化（PlayerCardStats.formatSpeed） ---------- */
export function formatSpeed(bytes) {
  if (bytes == null) return '0B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = Number(bytes);
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  if (Number.isInteger(value)) return `${value}${units[unit]}`;
  let s = value.toFixed(2);
  s = s.replace(/\.?0+$/, '');
  return `${s}${units[unit]}`;
}

/** 时长格式化：HH:MM:SS（有小时）或 MM:SS */
export function formatDuration(totalSeconds) {
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = totalSeconds % 60;
  const pad = (n) => String(n).padStart(2, '0');
  return h > 0 ? `${pad(h)}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

/* ---------- VersionUtil ---------- */
export function stripVPrefix(version) {
  return (version || '').replace(/^v/i, '');
}

/** 版本显示："当前 -> 最新" */
export function getVersionDisplayText(current, latest) {
  const cur = stripVPrefix(current);
  const lat = stripVPrefix(latest);
  if (lat && cur && lat !== cur) return `${cur} -> ${lat}`;
  return cur;
}

/** 比较版本（语义化，含 prerelease 段），a > b 返回 1 */
export function compareVersions(a, b) {
  const pa = stripVPrefix(a).split('-');
  const pb = stripVPrefix(b).split('-');
  const va = pa[0].split('.').map((x) => parseInt(x, 10) || 0);
  const vb = pb[0].split('.').map((x) => parseInt(x, 10) || 0);
  for (let i = 0; i < 3; i++) {
    if ((va[i] || 0) !== (vb[i] || 0)) return (va[i] || 0) > (vb[i] || 0) ? 1 : -1;
  }
  const ra = pa.slice(1);
  const rb = pb.slice(1);
  if (ra.length === 0 && rb.length === 0) return 0;
  if (ra.length === 0) return 1; // 无后缀 > 有后缀
  if (rb.length === 0) return -1;
  for (let i = 0; i < Math.max(ra.length, rb.length); i++) {
    const xa = ra[i];
    const xb = rb[i];
    if (xa === xb) continue;
    if (xa == null) return -1;
    if (xb == null) return 1;
    const na = parseInt(xa, 10);
    const nb = parseInt(xb, 10);
    if (!Number.isNaN(na) && !Number.isNaN(nb)) {
      if (na !== nb) return na > nb ? 1 : -1;
    } else {
      if (String(xa) > String(xb)) return 1;
      return -1;
    }
  }
  return 0;
}

/* ---------- 随机房间名（random_name.dart） ---------- */
const NAME_PREFIXES = [
  '月光', '星云', '量子', '琉璃', '蒸汽', '镜像', '蜂巢', '极光', '云端', '琥珀',
  '棱镜', '螺旋', '黄昏', '黎光', '数据', '像素', '立方', '穹顶', '虫洞', ' fractal',
  '相位', '回声', '虚像', '纳米', '拓扑', '弦月', '翡翠', '静滞', '折叠', '观星',
  '粒子', '相位', '虹膜', '熵减', '拓扑', '全息', '磁浮', '反物质', '曲速', '克莱因',
];
const NAME_SUFFIXES = [
  '会议室', '作战室', '温室', '音乐厅', '图书馆', '实验室', '观景台', '休息室',
  '指挥中心', '核心舱', '花园', '灯塔', '工坊', '档案馆', '放映厅', '竞技场',
  '画廊', '书房', '厨房', '天台', '地下室', '阁楼', '庭院', '瀑布旁', '湖心亭',
  '时空站', '轨道舱', '能量井', '传送门', '星门', '虫洞口', '水晶宫', '风暴眼',
  '寂静岭', '永夜城', '黎明要塞', '黄昏堡垒', '星尘平原', '虚空回廊', '回响室',
  '失落叶', '尽头',
];
export function randomRoomName() {
  const prefix = NAME_PREFIXES[Math.floor(Math.random() * NAME_PREFIXES.length)];
  const suffix = NAME_SUFFIXES[Math.floor(Math.random() * NAME_SUFFIXES.length)];
  return `${prefix}${suffix}`;
}

/** UUID v4 */
export function uuidV4() {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/** 随机 6 位数字（默认房间号/密码） */
export function randomSixDigits() {
  return String(Math.floor(100000 + Math.random() * 900000));
}
