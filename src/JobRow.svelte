<script lang="ts">
  import { CircleAlert, CircleCheck, RotateCw, Trash2, X } from "@lucide/svelte";
  import { cancelJob, removeJob, retryJob, type Job } from "./api";
  import { formatBytes, formatDuration, formatName, isVideoFormat } from "./format";
  import { t } from "./i18n";
  import Thumbnail from "./Thumbnail.svelte";

  let { job }: { job: Job } = $props();

  const running = $derived(job.status === "fetching" || job.status === "downloading" || job.status === "processing");
  const cancellable = $derived(running || job.status === "queued");
  const retryable = $derived(job.status === "failed" || job.status === "cancelled");
  const removable = $derived(retryable || job.status === "done");
  const percent = $derived(Math.round(job.progress * 100));
  const details = $derived.by(() => {
    const parts = [formatName(job.selection.format), job.label];
    if (job.size !== null) {
      parts.push(formatBytes(job.size));
    }
    if (job.status === "downloading") {
      parts.push(`${percent}%`);
      if (job.speed !== null) {
        parts.push(`${formatBytes(job.speed)}/s`);
      }
      if (job.eta !== null) {
        parts.push(t("remaining", { time: formatDuration(job.eta) }));
      }
    } else {
      parts.push(t(`status_${job.status}`));
    }
    return parts.join(" · ");
  });
</script>

<li class="row">
  <Thumbnail src={job.thumbnail} audio={!isVideoFormat(job.selection.format)} />
  <div class="content">
    <div class="title truncate">{job.title}</div>
    {#if running}
      <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={percent}>
        <div class="fill" style:width={`${percent}%`}></div>
      </div>
    {/if}
    {#if job.status === "failed"}
      <div class="muted danger line selectable">
        <CircleAlert size={14} aria-hidden="true" />
        <span class="truncate" title={job.error ?? undefined}>{t("status_failed")} · {job.error}</span>
      </div>
    {:else}
      <div class="muted line">
        {#if job.status === "done"}
          <CircleCheck size={14} aria-hidden="true" />
        {/if}
        <span class="truncate">{details}</span>
      </div>
    {/if}
  </div>
  <div class="actions">
    {#if retryable}
      <button class="icon-button" title={t("retry")} aria-label={t("retry")} onclick={() => retryJob(job.id)}>
        <RotateCw size={17} aria-hidden="true" />
      </button>
    {/if}
    {#if cancellable}
      <button class="icon-button" title={t("cancelDownload")} aria-label={t("cancelDownload")} onclick={() => cancelJob(job.id)}>
        <X size={17} aria-hidden="true" />
      </button>
    {/if}
    {#if removable}
      <button class="icon-button" title={t("remove")} aria-label={t("remove")} onclick={() => removeJob(job.id)}>
        <Trash2 size={17} aria-hidden="true" />
      </button>
    {/if}
  </div>
</li>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }

  .content {
    flex: 1;
    min-width: 0;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 2px;
  }

  .bar {
    height: 3px;
    margin-top: 6px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--border);
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s linear;
  }

  .actions {
    display: flex;
    flex: none;
    gap: 2px;
  }
</style>
