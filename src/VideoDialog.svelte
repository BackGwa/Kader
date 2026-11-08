<script lang="ts">
  import { AUDIO_FORMATS, VIDEO_FORMATS, enqueue, type Format, type Quality, type VideoInfo } from "./api";
  import Dialog from "./Dialog.svelte";
  import { formatBytes, formatDuration, formatName, isVideoFormat, qualityLabel } from "./format";
  import { t } from "./i18n";
  import Select from "./Select.svelte";
  import Thumbnail from "./Thumbnail.svelte";

  type Kind = "video" | "audio";

  let { info, onclose }: { info: VideoInfo; onclose: () => void } = $props();

  const formatsOf = (kind: Kind) =>
    (kind === "video" ? VIDEO_FORMATS : AUDIO_FORMATS).filter((format) => info.options[format]?.length);
  const firstFormat = () => [...formatsOf("video"), ...formatsOf("audio")][0];

  let format = $state<Format | undefined>(firstFormat());
  let qualityIndex = $state(0);
  let downloadButton = $state<HTMLButtonElement>();

  const kind = $derived<Kind>(format && !isVideoFormat(format) ? "audio" : "video");
  const qualities = $derived<Quality[]>(format ? (info.options[format] ?? []) : []);

  $effect(() => {
    downloadButton?.focus();
  });

  function selectKind(next: Kind) {
    format = formatsOf(next)[0];
    qualityIndex = 0;
  }

  function describe(quality: Quality) {
    const label = format ? qualityLabel(format, quality.value) : "";
    return [label, quality.codec, quality.size === null ? null : formatBytes(quality.size)]
      .filter(Boolean)
      .join(" · ");
  }

  async function download(event: SubmitEvent) {
    event.preventDefault();
    const quality = qualities[qualityIndex];
    if (format && quality) {
      await enqueue([
        {
          url: info.url,
          title: info.title,
          thumbnail: info.thumbnail,
          selection: { format, quality: quality.value },
          label: qualityLabel(format, quality.value),
        },
      ]);
      onclose();
    }
  }
</script>

<Dialog label={info.title} {onclose}>
  <form onsubmit={download}>
    <div class="heading">
      <Thumbnail src={info.thumbnail} width={112} />
      <div class="summary">
        <div class="title truncate" title={info.title}>{info.title}</div>
        {#if info.duration !== null}
          <div class="small muted tabular">{formatDuration(info.duration)}</div>
        {/if}
      </div>
    </div>

    {#if format}
      <div class="controls">
        <div class="segmented" role="group">
          {#each ["video", "audio"] as const as option (option)}
            <button
              type="button"
              aria-pressed={kind === option}
              disabled={formatsOf(option).length === 0}
              onclick={() => selectKind(option)}
            >
              {t(option)}
            </button>
          {/each}
        </div>
        <Select bind:value={format} label={t("format")} width={92} onchange={() => (qualityIndex = 0)}>
          {#each formatsOf(kind) as option (option)}
            <option value={option}>{formatName(option)}</option>
          {/each}
        </Select>
        <Select bind:value={qualityIndex} label={t("quality")}>
          {#each qualities as quality, index (index)}
            <option value={index}>{describe(quality)}</option>
          {/each}
        </Select>
      </div>
    {:else}
      <p class="small muted">{t("noFormats")}</p>
    {/if}

    <div class="dialog-actions">
      <button type="button" class="secondary" onclick={onclose}>{t("cancel")}</button>
      <button bind:this={downloadButton} type="submit" class="primary" disabled={!format}>{t("download")}</button>
    </div>
  </form>
</Dialog>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
  }

  .heading {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }

  .summary {
    flex: 1;
    min-width: 0;
  }

  .summary .title {
    font-size: var(--text-lg);
  }

  .controls {
    display: flex;
    gap: var(--space-2);
  }

  p {
    margin: 0;
  }
</style>
