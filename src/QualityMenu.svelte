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

  const MENU_MARGIN = 4;
  const ESTIMATED_MENU_HEIGHT = 240;

  let trigger: HTMLButtonElement;
  let open = $state(false);
  let position = $state("");

  const qualities = $derived(options?.[format] ?? []);
  const measured = $derived(qualities.filter((quality) => quality.value !== null));

  function toggle(event: MouseEvent) {
    event.stopPropagation();
    if (open) {
      open = false;
      return;
    }
    const rect = trigger.getBoundingClientRect();
    const right = window.innerWidth - rect.right;
    position =
      rect.bottom + ESTIMATED_MENU_HEIGHT > window.innerHeight
        ? `bottom:${window.innerHeight - rect.top + MENU_MARGIN}px;right:${right}px`
        : `top:${rect.bottom + MENU_MARGIN}px;right:${right}px`;
    open = true;
    onopen();
  }

  function pick(next: number | null) {
    onselect(next);
    open = false;
  }

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      open = false;
      trigger.focus();
    }
  }
</script>

<svelte:window onclick={() => (open = false)} />

<div class="menu-root">
  <button
    bind:this={trigger}
    type="button"
    class="trigger"
    aria-label={t("quality")}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={toggle}
  >
    <span>{qualityLabel(format, value)}</span>
    <ChevronDown size={14} aria-hidden="true" />
  </button>

  {#if open}
    <div class="menu" role="menu" tabindex="-1" style={position} onkeydown={closeOnEscape} onclick={(event) => event.stopPropagation()}>
      <button type="button" role="menuitem" class="item" onclick={() => pick(null)}>
        <span>{t("best")}</span>
      </button>
      {#if loading}
        <div class="status muted">
          <LoaderCircle class="spin" size={14} aria-hidden="true" />
          {t("qualityLoading")}
        </div>
      {:else if failed}
        <div class="status danger">{t("qualityFailed")}</div>
      {:else if options && measured.length === 0}
        <div class="status muted">{t("qualityUnavailable")}</div>
      {:else}
        {#each measured as quality (quality.value)}
          <button type="button" role="menuitem" class="item" onclick={() => pick(quality.value)}>
            <span>{qualityLabel(format, quality.value)}{quality.codec ? ` · ${quality.codec}` : ""}</span>
            {#if quality.size !== null}
              <span class="muted">{t("about", { size: formatBytes(quality.size) })}</span>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
    width: 92px;
    height: 32px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: 7px;
    background: var(--background);
  }

  .menu {
    position: fixed;
    z-index: 10;
    width: 220px;
    max-height: 240px;
    overflow-y: auto;
    padding: 4px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--background);
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    min-height: 30px;
    padding: 0 8px;
    border: 0;
    border-radius: 5px;
    background: transparent;
  }

  .item:hover {
    background: var(--hover);
  }

  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 8px;
    font-size: 12px;
  }

  .menu :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
