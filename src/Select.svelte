<script lang="ts" generics="T">
  import { ChevronDown } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  let {
    value = $bindable(),
    label,
    width,
    onchange,
    children,
  }: {
    value: T;
    label: string;
    width?: number;
    onchange?: () => void;
    children: Snippet;
  } = $props();
</script>

<div class="select" class:fill={width === undefined} style:width={width === undefined ? undefined : `${width}px`}>
  <select aria-label={label} bind:value {onchange}>
    {@render children()}
  </select>
  <ChevronDown size={14} aria-hidden="true" />
</div>

<style>
  .select {
    position: relative;
    display: flex;
    flex: none;
    align-items: center;
    color: var(--muted);
  }

  .select.fill {
    flex: 1;
    min-width: 0;
  }

  select {
    width: 100%;
    height: var(--control-height);
    padding: 0 var(--space-5) 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--field);
    color: var(--fg);
    appearance: none;
    -webkit-appearance: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  select:hover {
    background: var(--field-hover);
  }

  .select :global(svg) {
    position: absolute;
    right: var(--space-2);
    pointer-events: none;
  }
</style>
