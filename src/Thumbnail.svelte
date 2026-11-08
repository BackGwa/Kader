<script lang="ts">
  let { src, width = 80 }: { src: string | null; width?: number } = $props();

  let failed = $state(false);
</script>

<div class="thumbnail" style:width={`${width}px`}>
  {#if src && !failed}
    <img {src} alt="" loading="lazy" referrerpolicy="no-referrer" onerror={() => (failed = true)} />
  {/if}
</div>

<style>
  .thumbnail {
    flex: none;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: var(--radius-md);
    background: var(--field);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
