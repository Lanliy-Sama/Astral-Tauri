/**
 * 深链接处理 —— 对应 lib/core/app_links/（AppLinkRegistry）。
 * astral://room?code=<分享码> → 导入房间并跳转；
 * astral://debug → 调试提示。
 */
import { isTauri } from '../utils/platform.js';
import { sm } from './service-manager.svelte.js';
import { decryptRoom, validateRoom, cleanRoom } from '../utils/roomShareCodec.js';
import { snackError, snackSuccess, snackInfo } from '../components/common/snackbars.svelte.js';

async function handleLink(url) {
  try {
    const uri = new URL(url);
    if (uri.protocol !== 'astral:' || !uri.host) return;

    if (uri.host === 'room') {
      const code = uri.searchParams.get('code');
      const cleaned = (code || '').replace(/\s+/g, '');
      if (!cleaned) {
        snackError('分享链接格式错误', '不是有效的房间分享链接');
        return;
      }
      if (cleaned.length < 10) {
        snackError('分享码无效', '分享码解析失败');
        return;
      }
      const room = await decryptRoom(cleaned);
      if (!room) {
        snackError('分享码解析失败', '无法解析房间信息');
        return;
      }
      const [valid, err] = validateRoom(room);
      if (!valid) {
        snackError('房间信息不完整', err || '房间数据无效');
        return;
      }
      /* 查重（加密房间比 name+roomName+password；非加密比 roomName+password） */
      const duplicate = sm.roomState.rooms.some((r) =>
        room.encrypted
          ? r.name === room.name && r.roomName === room.roomName && r.password === room.password
          : r.roomName === room.roomName && r.password === room.password
      );
      if (duplicate) {
        snackInfo('房间已存在', room.name);
        return;
      }
      const cleanedRoom = cleanRoom(room);
      const saved = await sm.addRoom(cleanedRoom);
      sm.uiState.goTo(1);
      await sm.setRoom(saved);
      snackSuccess('导入成功', `已成功添加并选中房间"${room.name}"`);
    } else if (uri.host === 'debug') {
      snackInfo('调试信息', '已触发调试链接： debug');
    }
  } catch (e) {
    snackError('处理分享链接失败', String(e));
  }
}

export async function initAppLinks() {
  if (!isTauri()) return;
  try {
    const deepLink = await import('@tauri-apps/plugin-deep-link');
    /* 冷启动链接 */
    const initial = await deepLink.getCurrent();
    if (initial) handleLink(initial);
    /* 运行中链接 */
    await deepLink.onDeepLink((event) => {
      const url = Array.isArray(event?.urls) ? event.urls[0] : event?.url;
      if (typeof url === 'string') handleLink(url);
    });
  } catch (e) {
    console.warn('深链接初始化失败:', e);
  }
}
