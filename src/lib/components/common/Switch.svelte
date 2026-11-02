<script>
  /** M3 Switch，对应 Flutter Switch（Flutter 默认不在 thumb 上绘制任何图标，
   *  仅在显式传入 thumbIcon 时才绘制；Astral 原版未传入，故无 √） */
  let { value = $bindable(false), disabled = false, scale = false, onChange = null } = $props();

  function toggle() {
    if (disabled) return;
    value = !value;
    onChange?.(value);
  }
</script>

<button
  class="md-switch {value ? 'on' : ''} {scale ? 'scale-08' : ''}"
  class:disabled
  aria-checked={value}
  role="switch"
  onclick={toggle}
  tabindex={disabled ? -1 : 0}
  {disabled}
>
  <span class="thumb"></span>
</button>
