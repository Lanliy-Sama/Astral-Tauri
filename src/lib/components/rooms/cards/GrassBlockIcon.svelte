<script>
  let { size = 24, class: className = '' } = $props();
</script>

<svg
  class={className}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
  shape-rendering="crispEdges"
>
  <!-- Classic Minecraft grass block: green pixel top and earthy sides. -->
  <path d="M12 1 23 6.5 12 12 1 6.5 12 1Z" fill="#69A844" />
  <path d="M1 6.5 12 12v11L1 17V6.5Z" fill="#7B5636" />
  <path d="m12 12 11-5.5V17l-11 6V12Z" fill="#68452D" />
  <path d="m1 6.5 11 5.5v3L1 9.5v-3Z" fill="#5B9639" />
  <path d="m12 12 11-5.5v3l-11 5.5v-3Z" fill="#4D8533" />
  <path d="m5 4 3 1.5v2L5 6V4Zm6-1 2 1-2 1-2-1 2-1Zm5 3 3 1.5v2l-3-1.5V6ZM3 8l2 1v2L3 10V8Zm5 2 2 1v2L8 12v-2Z" fill="#86C454" />
  <path d="M3 12h2v2H3v-2Zm4 3h2v2H7v-2Zm-3 3h2v2H4v-2Zm11-4h2v2h-2v-2Zm4-3h2v2h-2v-2Zm-3 7h2v2h-2v-2Zm6-4h2v2h-2v-2Z" fill="#916846" />
</svg>
