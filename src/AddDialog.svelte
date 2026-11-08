<script lang="ts">
  import { onMount } from "svelte";
  import { AUDIO_FORMATS, VIDEO_FORMATS, enqueue, getSettings, type Format, type Quality, type VideoInfo } from "./api";
  import Dialog from "./Dialog.svelte";
  import { formatBytes, formatDuration, formatName, isVideoFormat, qualityLabel } from "./format";
  import { t } from "./i18n";
  import Thumbnail from "./Thumbnail.svelte";

  type Kind = "video" | "audio";

  let { info, onclose }: { info: VideoInfo; onclose: () => void } = $props();

  const available = (formats: Format[]) => formats.filter((format) => info.options[format]?.length);
  const formatsOf = (kind: Kind) => available(kind === "video" ? VIDEO_FORMATS : AUDIO_FORMATS);

  const kinds = $derived((["video", "audio"] as Kind[]).filter((kind) => formatsOf(kind).length > 0));
  const initialFormat = [...formatsOf("video"), ...formatsOf("audio")][0];

  let format = $state<Format | undefined>(initialFormat);
  let qualityIndex = $state(0);
  let saveDir = $state("");

  const kind = $derived<Kind>(format && !isVideoFormat(format) ? "audio" : "video");
  const qualities = $derived<Quality[]>(format ? (info.options[format] ?? []) : []);

  onMount(async () => {
    saveDir = (await getSettings()).saveDir;
  });

  function selectKind(event: Event) {
    format = formatsOf((event.currentTarget as HTMLSelectElement).value as Kind)[0];
    qualityIndex = 0;
  }

  function selectFormat(event: Event) {
    format = (event.currentTarget as HTMLSelectElement).value as Format;
    qualityIndex = 0;
  }

  function describe(quality: Quality) {
    const label = format ? qualityLabel(format, quality.value) : "";
    return quality.codec ? `${label} · ${quality.codec}` : label;
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const quality = qualities[qualityIndex];
    if (!format || !quality) {
      return;
    }
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
</script>

<Dialog label={info.title} {onclose}>
  <form class="dialog-body" onsubmit={submit}>
    <div class="header">
      <Thumbnail src={info.thumbnail} audio={kind === "audio"} />
      <div class="summary">
        <div class="title truncate" title={info.title}>{info.title}</div>
        {#if info.duration !== null}
          <div class="muted">{formatDuration(info.duration)}</div>
        {/if}
      </div>
    </div>

    {#if format}
      <div class="fields">
        <select aria-label={t("kind")} value={kind} onchange={selectKind}>
          {#each kinds as option (option)}
            <option value={option}>{t(option)}</option>
          {/each}
        </select>
        <select aria-label={t("format")} value={format} onchange={selectFormat}>
          {#each formatsOf(kind) as option (option)}
            <option value={option}>{formatName(option)}</option>
          {/each}
        </select>
      </div>

      <div class="qualities" role="radiogroup" aria-label={t("quality")}>
        {#each qualities as quality, index (index)}
          <label class="quality">
            <input type="radio" name="quality" value={index} bind:group={qualityIndex} />
            <span class="name">{describe(quality)}</span>
            {#if quality.size !== null}
              <span class="muted">{t("about", { size: formatBytes(quality.size) })}</span>
            {/if}
          </label>
        {/each}
      </div>

      <div class="muted truncate" title={saveDir}>{t("saveTo", { path: saveDir })}</div>
    {:else}
      <div class="muted">{t("noFormats")}</div>
    {/if}

    <div class="dialog-actions">
      <button type="button" class="button" onclick={onclose}>{t("cancel")}</button>
      <button type="submit" class="button primary" disabled={!format}>{t("download")}</button>
    </div>
  </form>
</Dialog>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .summary {
    flex: 1;
    min-width: 0;
  }

  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .qualities {
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 7px;
  }

  .quality {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 0 10px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
  }

  .quality:last-child {
    border-bottom: 0;
  }

  .quality:hover {
    background: var(--hover);
  }

  .name {
    flex: 1;
  }

  input {
    margin: 0;
    accent-color: var(--accent);
  }
</style>
