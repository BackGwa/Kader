<script lang="ts">
  import { ChevronDown, LoaderCircle } from "@lucide/svelte";
  import type { Format, QualityOptions } from "./api";
  import { formatBytes, qualityLabel } from "./format";
  import { t } from "./i18n";

  let {
    format,
    value,
    options,
    loading,
    failed,
    onopen,
    onselect,
  }: {
    format: Format;
    value: number | null;
    options: QualityOptions | null;
    loading: boolean;
    failed: boolean;
    onopen: () => void;
    onselect: (value: number | null) => void;
  } = $props();

  const MENU_GAP = 4;
  const MENU_MAX_HEIGHT = 240;

  let trigger: HTMLButtonElement;
  let open = $state(false);
  let position = $state("");

  const measured = $derived((options?.[format] ?? []).filter((quality) => quality.value !== null));

  function toggle(event: MouseEvent) {
    event.stopPropagation();
    if (open) {
      open = false;
      return;
    }
    const rect = trigger.getBoundingClientRect();
    const right = window.innerWidth - rect.right;
    position =
      rect.bottom + MENU_MAX_HEIGHT > window.innerHeight
        ? `bottom:${window.innerHeight - rect.top + MENU_GAP}px;right:${right}px`
        : `top:${rect.bottom + MENU_GAP}px;right:${right}px`;
    open = true;
    onopen();
  }

  function pick(next: number | null) {
    onselect(next);
    open = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      open = false;
      trigger.focus();
    }
  }
</script>

<svelte:window onclick={() => (open = false)} />

<div class="root">
  <button
    bind:this={trigger}
    type="button"
    class="trigger"
    aria-label={t("quality")}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={toggle}
  >
    <span class="truncate">{qualityLabel(format, value)}</span>
    <ChevronDown size={14} aria-hidden="true" />
  </button>

  {#if open}
    <div class="menu" role="menu" tabindex="-1" style={position} onkeydown={handleKeydown} onclick={(event) => event.stopPropagation()}>
      <button type="button" role="menuitem" class="item" onclick={() => pick(null)}>{t("best")}</button>
      {#if loading}
        <div class="status"><LoaderCircle class="spin" size={16} aria-hidden="true" /></div>
      {:else if failed}
        <div class="status">{t("qualityFailed")}</div>
      {:else if options && measured.length === 0}
        <div class="status">{t("qualityUnavailable")}</div>
      {:else}
        {#each measured as quality (quality.value)}
          <button type="button" role="menuitem" class="item" onclick={() => pick(quality.value)}>
            <span>{qualityLabel(format, quality.value)}{quality.codec ? ` · ${quality.codec}` : ""}</span>
            {#if quality.size !== null}
              <span class="small muted tabular">{formatBytes(quality.size)}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .root {
    flex: none;
  }

  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-1);
    width: 104px;
    height: var(--control-height);
    padding: 0 var(--space-2) 0 var(--space-3);
    border-radius: var(--radius-md);
    background: var(--field);
  }

  .trigger :global(svg) {
    flex: none;
    color: var(--muted);
  }

  .trigger:hover {
    background: var(--field-hover);
  }

  .menu {
    position: fixed;
    z-index: 30;
    width: 210px;
    max-height: 240px;
    overflow-y: auto;
    padding: var(--control-inset);
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--raised);
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    height: var(--control-inner-height);
    padding: 0 var(--space-3);
    border-radius: var(--radius-sm);
    text-align: left;
  }

  .item:hover {
    background: var(--field);
  }

  .status {
    display: flex;
    align-items: center;
    min-height: var(--control-inner-height);
    padding: 0 var(--space-3);
    color: var(--muted);
    font-size: var(--text-sm);
  }
</style>
