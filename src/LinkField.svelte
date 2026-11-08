<script lang="ts">
  import { ArrowRight, Link, LoaderCircle, X } from "@lucide/svelte";
  import { linkFrom } from "./format";
  import { t } from "./i18n";

  let {
    value = $bindable(),
    busy,
    onanalyze,
    onclear,
  }: {
    value: string;
    busy: boolean;
    onanalyze: (text: string) => void;
    onclear: () => void;
  } = $props();

  let input: HTMLInputElement;

  export function focus() {
    input.focus();
  }

  function handlePaste(event: ClipboardEvent) {
    const text = event.clipboardData?.getData("text") ?? "";
    if (linkFrom(text)) {
      event.preventDefault();
      value = text.trim();
      onanalyze(value);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      onanalyze(value);
    } else if (event.key === "Escape") {
      onclear();
    }
  }
</script>

<div class="field">
  {#if busy}
    <LoaderCircle class="spin" size={16} aria-hidden="true" />
  {:else}
    <Link size={16} aria-hidden="true" />
  {/if}
  <input
    bind:this={input}
    bind:value
    type="text"
    aria-label={t("linkPlaceholder")}
    placeholder={t("linkPlaceholder")}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
    onpaste={handlePaste}
    onkeydown={handleKeydown}
  />
  {#if busy}
    <button type="button" aria-label={t("cancel")} title={t("cancel")} onclick={onclear}>
      <X size={16} aria-hidden="true" />
    </button>
  {:else if value.trim()}
    <button type="button" aria-label={t("startAnalysis")} title={t("startAnalysis")} onclick={() => onanalyze(value)}>
      <ArrowRight size={16} aria-hidden="true" />
    </button>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    height: var(--control-height);
    padding: 0 var(--control-inset) 0 var(--space-3);
    border-radius: var(--radius-md);
    background: var(--field);
    color: var(--muted);
  }

  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--fg);
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }

  input::placeholder {
    color: var(--muted);
  }

  button {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: var(--control-inner-height);
    height: var(--control-inner-height);
    border-radius: var(--radius-sm);
    color: var(--muted);
  }

  button:hover {
    background: var(--field-hover);
    color: var(--fg);
  }
</style>
