<script module>
/** 子页面路由（Svelte 5 模块）—— 对应 Flutter 根 Navigator 的 push/pop。 */
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
  const top = stack[stack.length - 1];
  if (!top || top.closing) return;
  /* 退场动画：标记 closing（route-out 190ms）后再真正移除；期间的
   * 再次 pop 被 closing 守卫挡住，不会连弹两层 */
  top.closing = true;
  setTimeout(() => {
    const idx = stack.indexOf(top);
    if (idx >= 0) stack.splice(idx, 1);
  }, 190);
}

export function getStack() {
  return stack;
}

export function clearPages() {
  stack.length = 0;
}
</script>
