/**
 * Snackbar 系统 —— 对应 AppSnackBars。
 * floating 行为；error 红 / success 绿 / info 蓝；图标白色、标题加粗。
 */
import { writeClipboard } from '../../rust/api.js';

const state = $state({
  queue: [],
});

let seq = 0;

function push(snack) {
  const id = ++seq;
  state.queue.push({ id, ...snack });
  if (state.queue.length > 3) state.queue.splice(0, state.queue.length - 3);
  const duration = snack.duration ?? (snack.type === 'error' ? 4000 : 3000);
  setTimeout(() => dismiss(id), duration);
}

function dismiss(id) {
  const idx = state.queue.findIndex((s) => s.id === id);
  if (idx >= 0) state.queue.splice(idx, 1);
}

export function getSnackbars() {
  return state.queue;
}

export function showSnackbar({ title, message = '', type = 'info', icon, duration, action, bg = null }) {
  const icons = { error: 'error_outline', success: 'check_circle_outline', info: 'info_outline' };
  push({ title, message, type, icon: icon || icons[type] || 'info_outline', duration, action, bg });
}

export function snackError(title, message, opts = {}) {
  showSnackbar({ title, message, type: 'error', ...opts });
}

export function snackSuccess(title, message, opts = {}) {
  showSnackbar({ title, message, type: 'success', ...opts });
}

export function snackInfo(title, message, opts = {}) {
  showSnackbar({ title, message, type: 'info', ...opts });
}

/** 带复制错误操作的错误条（AppSnackBars.error copyAction） */
export function snackErrorCopy(title, message) {
  showSnackbar({
    title,
    message,
    type: 'error',
    duration: 4000,
    action: {
      label: '复制错误',
      onPressed: async () => {
        await writeClipboard(`${title}: ${message}`);
      },
    },
  });
}
