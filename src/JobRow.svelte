<script lang="ts">
  import { Check, FolderOpen, RotateCw, Trash2, X } from "@lucide/svelte";
  import { cancelJob, removeJob, retryJob, revealJob, type Job } from "./api";
  import { formatBytes, formatName } from "./format";
  import { t } from "./i18n";
  import Thumbnail from "./Thumbnail.svelte";

  let { job, onfailure }: { job: Job; onfailure: (message: string) => void } = $props();

  const downloading = $derived(job.status === "downloading");
  const cancellable = $derived(job.status === "queued" || job.status === "fetching" || downloading || job.status === "processing");
  const retryable = $derived(job.status === "failed" || job.status === "cancelled");
  const removable = $derived(retryable || job.status === "done");
  const percent = $derived(Math.round(job.progress * 100));
  const details = $derived(
    [formatName(job.selection.format), job.label, job.size === null ? null : formatBytes(job.size)]
      .filter(Boolean)
      .join(" · "),
  );
  const state = $derived.by(() => {
    if (downloading) {
      return job.speed === null ? `${percent}%` : `${percent}% · ${formatBytes(job.speed)}/s`;
    }
    return t(`status_${job.status}`);
  });

  function reveal() {
    revealJob(job.id).catch((error) => onfailure(String(error)));
  }
</script>

<li>
  <Thumbnail src={job.thumbnail} />

  <div class="content">
    <div class="truncate name" title={job.title}>{job.title}</div>
    {#if job.status === "failed" && job.error}
      <div class="small muted truncate selectable" title={job.error}>{job.error}</div>
    {:else}
      <div class="small muted truncate">{details}</div>
    {/if}
    {#if downloading}
      <div class="track" role="progressbar" aria-label={job.title} aria-valuemin="0" aria-valuemax="100" aria-valuenow={percent}>
        <div class="fill" style:width={`${percent}%`}></div>
      </div>
    {/if}
  </div>

  <div class="trailing">
    <div class="state small muted tabular">
      {#if job.status === "done"}
        <Check size={16} aria-label={state} />
      {:else}
        {state}
      {/if}
    </div>
    <div class="actions">
      {#if retryable}
        <button class="icon-button" title={t("retry")} aria-label={t("retry")} onclick={() => retryJob(job.id)}>
          <RotateCw size={16} aria-hidden="true" />
        </button>
      {/if}
      {#if cancellable}
        <button class="icon-button" title={t("cancelDownload")} aria-label={t("cancelDownload")} onclick={() => cancelJob(job.id)}>
          <X size={16} aria-hidden="true" />
        </button>
      {/if}
      {#if job.status === "done"}
        <button class="icon-button" title={t("showInFolder")} aria-label={t("showInFolder")} onclick={reveal}>
          <FolderOpen size={16} aria-hidden="true" />
        </button>
      {/if}
      {#if removable}
        <button class="icon-button" title={t("remove")} aria-label={t("remove")} onclick={() => removeJob(job.id)}>
          <Trash2 size={16} aria-hidden="true" />
        </button>
      {/if}
    </div>
  </div>
</li>

<style>
  li {
    display: grid;
    grid-template-columns: 80px minmax(0, 1fr) 128px;
    gap: var(--space-3);
    align-items: center;
    min-height: 64px;
    padding: var(--space-2) 0;
  }

  .content {
    min-width: 0;
  }

  .name {
    font-weight: var(--weight-medium);
  }

  .track {
    height: 2px;
    margin-top: var(--space-2);
    overflow: hidden;
    background: var(--track);
  }

  .fill {
    height: 100%;
    background: var(--fg);
    transition: width 0.25s linear;
  }

  .trailing {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    min-height: var(--control-height);
  }

  .state {
    display: flex;
    align-items: center;
    white-space: nowrap;
  }

  .actions {
    position: absolute;
    top: 0;
    right: 0;
    display: none;
  }

  li:hover .actions,
  .actions:focus-within {
    display: flex;
  }

  li:hover .state {
    visibility: hidden;
  }

  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
