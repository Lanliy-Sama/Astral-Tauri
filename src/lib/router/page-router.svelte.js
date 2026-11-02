/**
 * 子页面路由 —— 对应 Flutter 根 Navigator 的 push/pop。
 * MainScreen 在最上层渲染路由栈（覆盖整个壳，与原版行为一致）。
 */

let seq = 0;
const stack = $state([]);

export function pushPage(component, props = {}) {
  stack.push({ id: ++seq, component, props });
}

export function popPage() {
  stack.pop();
}

export function getStack() {
  return stack;
}

export function clearPages() {
  stack.length = 0;
}
