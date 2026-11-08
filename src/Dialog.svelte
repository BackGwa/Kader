<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    label,
    wide = false,
    onclose,
    children,
  }: { label: string; wide?: boolean; onclose: () => void; children: Snippet } = $props();

  let element: HTMLDialogElement;

  $effect(() => {
    element.showModal();
  });

  function handleCancel(event: Event) {
    event.preventDefault();
    onclose();
  }
</script>

<dialog bind:this={element} class:wide aria-label={label} oncancel={handleCancel}>
  {@render children()}
</dialog>
