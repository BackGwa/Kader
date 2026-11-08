<script lang="ts">
  import { onDestroy } from "svelte";
  import {
    AUDIO_FORMATS,
    VIDEO_FORMATS,
    cancelProbe,
    enqueue,
    newToken,
    probe,
    type Format,
    type PlaylistEntry,
    type PlaylistInfo,
    type QualityOptions,
  } from "./api";
  import Dialog from "./Dialog.svelte";
  import { formatDuration, formatName, qualityLabel } from "./format";
  import { t } from "./i18n";
  import QualityMenu from "./QualityMenu.svelte";
  import Select from "./Select.svelte";
  import Thumbnail from "./Thumbnail.svelte";

  type Row = {
    entry: PlaylistEntry;
    selected: boolean;
    format: Format;
    quality: number | null;
    options: QualityOptions | null;
    loading: boolean;
    failed: boolean;
    token: number | null;
  };

  const DEFAULT_FORMAT: Format = "mp4";
  const FORMATS = [...VIDEO_FORMATS, ...AUDIO_FORMATS];

  let { info, onclose }: { info: PlaylistInfo; onclose: () => void } = $props();

  const createRows = (): Row[] =>
    info.entries.map((entry) => ({
      entry,
      selected: true,
      format: DEFAULT_FORMAT,
      quality: null,
      options: null,
      loading: false,
      failed: false,
      token: null,
    }));

  let rows = $state<Row[]>(createRows());
  let sharedFormat = $state<Format>(DEFAULT_FORMAT);

  const selectedRows = $derived(rows.filter((row) => row.selected));
  const allSelected = $derived(selectedRows.length === rows.length);

  let downloadButton = $state<HTMLButtonElement>();

  $effect(() => {
    downloadButton?.focus();
  });

  onDestroy(() => {
    rows.forEach((row) => {
      if (row.token !== null) {
        cancelProbe(row.token);
      }
    });
  });

  function reconcileQuality(row: Row) {
    const stillOffered = row.options?.[row.format]?.some((quality) => quality.value === row.quality);
    if (!stillOffered) {
      row.quality = null;
    }
  }

  function applySharedFormat() {
    rows.forEach((row) => {
      row.format = sharedFormat;
      reconcileQuality(row);
    });
  }

  function toggleAll() {
    const next = !allSelected;
    rows.forEach((row) => (row.selected = next));
  }

  async function loadQualities(row: Row) {
    if (row.options || row.loading) {
      return;
    }
    const token = newToken();
    row.loading = true;
    row.failed = false;
    row.token = token;
    try {
      const result = await probe(token, row.entry.url);
      if (result.kind === "video") {
        row.options = result.options;
      } else {
        row.failed = true;
      }
    } catch {
      row.failed = true;
    } finally {
      row.loading = false;
      row.token = null;
    }
  }

  async function download() {
    await enqueue(
      selectedRows.map((row) => ({
        url: row.entry.url,
        title: row.entry.title,
        thumbnail: row.entry.thumbnail,
        selection: { format: row.format, quality: row.quality },
        label: qualityLabel(row.format, row.quality),
      })),
    );
    onclose();
  }
</script>

<Dialog label={info.title} wide {onclose}>
  <div class="layout">
    <header>
      <div class="title truncate" title={info.title}>{info.title}</div>
      <Select bind:value={sharedFormat} label={t("format")} width={92} onchange={applySharedFormat}>
        {#each FORMATS as format (format)}
          <option value={format}>{formatName(format)}</option>
        {/each}
      </Select>
    </header>

    <ul>
      {#each rows as row, index (index)}
        <li>
          <input type="checkbox" aria-label={row.entry.title} bind:checked={row.selected} />
          <Thumbnail src={row.entry.thumbnail} width={64} />
          <div class="truncate" title={row.entry.title}>{row.entry.title}</div>
          <span class="small muted tabular duration">
            {row.entry.duration === null ? "" : formatDuration(row.entry.duration)}
          </span>
          <Select bind:value={row.format} label={t("format")} width={92} onchange={() => reconcileQuality(row)}>
            {#each FORMATS as format (format)}
              <option value={format}>{formatName(format)}</option>
            {/each}
          </Select>
          <QualityMenu
            format={row.format}
            value={row.quality}
            options={row.options}
            loading={row.loading}
            failed={row.failed}
            onopen={() => loadQualities(row)}
            onselect={(value) => (row.quality = value)}
          />
        </li>
      {/each}
    </ul>

    <footer>
      <label>
        <input type="checkbox" checked={allSelected} onchange={toggleAll} />
        <span>{t("selectAll")}</span>
      </label>
      <div class="dialog-actions">
        <button type="button" class="secondary" onclick={onclose}>{t("cancel")}</button>
        <button bind:this={downloadButton} type="button" class="primary tabular" disabled={selectedRows.length === 0} onclick={download}>
          {t("download")} {selectedRows.length}
        </button>
      </div>
    </footer>
  </div>
</Dialog>

<style>
  .layout {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-height: calc(100vh - 50px);
    padding: var(--space-5);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  header .title {
    flex: 1;
    min-width: 0;
    font-size: var(--text-lg);
  }

  ul {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  li {
    display: grid;
    grid-template-columns: 15px 64px minmax(0, 1fr) 44px 92px 104px;
    gap: var(--space-3);
    align-items: center;
    height: 52px;
  }

  .duration {
    text-align: right;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
</style>
