<script>
  import Icon from './Icon.svelte';
  import { getSnackbars } from './SnackbarStore.svelte';

  const snacks = $derived(getSnackbars());
</script>

<div class="md-snackbar-host">
  {#each snacks as snack (snack.id)}
    <div
      class="md-snackbar {snack.type}"
      class:snack-out={snack.closing}
      style={snack.bg ? `background:${snack.bg}` : ''}
    >
      <span class="snack-icon"><Icon name={snack.icon} size={22} color="#fff" /></span>
      <div class="snack-body">
        <div class="snack-title">{snack.title}</div>
        {#if snack.message}
          <div class="snack-message">{snack.message}</div>
        {/if}
      </div>
      {#if snack.action}
        <button class="snack-action" onclick={() => snack.action.onPressed()}>{snack.action.label}</button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .md-snackbar.snack-out {
    animation: snackbar-out 200ms ease forwards;
    pointer-events: none;
  }
  @keyframes snackbar-out {
    from { transform: translateY(0); opacity: 1; }
    to { transform: translateY(16px); opacity: 0; }
  }
</style>
