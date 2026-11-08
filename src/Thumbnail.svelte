<script lang="ts">
  import { Music, Play } from "@lucide/svelte";

  let { src, audio = false }: { src: string | null; audio?: boolean } = $props();

  let failed = $state(false);
</script>

<div class="thumbnail">
  {#if src && !failed}
    <img {src} alt="" loading="lazy" referrerpolicy="no-referrer" onerror={() => (failed = true)} />
  {:else if audio}
    <Music size={20} aria-hidden="true" />
  {:else}
    <Play size={20} aria-hidden="true" />
  {/if}
</div>
