<script>
  /**
   * M3 OutlinedTextField —— 支持浮动标签、前后置图标、错误文本、禁用。
   */
  import Icon from './Icon.svelte';

  let {
    value = $bindable(''),
    label = '',
    hint = '',
    placeholder = '',
    prefixIcon = '',
    suffixIcon = '',
    errorText = '',
    helperText = '',
    disabled = false,
    type = 'text',
    multiline = false,
    rows = 3,
    autofocus = false,
    dense = false,
    inputmode = null,
    onChange = () => {},
    onEnter = null,
    floatAlways = false,
    style = '',
  } = $props();

  let focused = $state(false);

  const hasLabel = $derived(label !== '');
  const floating = $derived(
    focused || value !== '' || floatAlways || placeholder !== ''
  );

  function handleInput(e) {
    value = e.target.value;
    onChange(value);
  }
</script>

<div
  class="md-text-field {focused ? 'focused' : ''} {errorText ? 'error' : ''} {disabled ? 'disabled' : ''} {dense ? 'dense' : ''}"
  style={style}
>
  {#if prefixIcon}
    <span class="tf-prefix"
      ><Icon
        name={prefixIcon}
        size={24}
        color={errorText ? 'var(--md-error)' : focused ? 'var(--md-primary)' : 'var(--md-on-surface-variant)'}
    /></span>
  {/if}
  <div class="tf-input-wrap">
    {#if multiline}
      <textarea
        {rows}
        {disabled}
        {placeholder}
        {autofocus}
        onfocus={() => (focused = true)}
        onblur={() => (focused = false)}
        oninput={handleInput}>{value}</textarea>
    {:else}
      <input
        {type}
        {disabled}
        {placeholder}
        {autofocus}
        {inputmode}
        value={value}
        onfocus={() => (focused = true)}
        onblur={() => (focused = false)}
        oninput={handleInput}
        onkeydown={(e) => {
          if (e.key === 'Enter' && onEnter) onEnter(value);
        }}
      />
    {/if}
    {#if hasLabel}
      <label class="tf-label" class:floated={floating}>{label}</label>
    {/if}
  </div>
  {#if suffixIcon}
    <span class="tf-suffix"><Icon name={suffixIcon} size={24} /></span>
  {/if}
</div>
{#if errorText}
  <div class="tf-error-text">{errorText}</div>
{:else if helperText}
  <div class="tf-helper">{helperText}</div>
{/if}
