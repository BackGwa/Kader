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

  onDestroy(() => {
    rows.forEach((row) => {
      if (row.token !== null) {
        cancelProbe(row.token);
      }
    });
  });

  function setFormat(row: Row, format: Format) {
    row.format = format;
    const stillOffered = row.options?.[format]?.some((quality) => quality.value === row.quality);
    if (!stillOffered) {
      row.quality = null;
    }
  }

  function applySharedFormat() {
    rows.forEach((row) => setFormat(row, sharedFormat));
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

  async function addSelected() {
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
  <div class="dialog-body">
    <div class="header">
      <div class="summary">
        <div class="title truncate" title={info.title}>{info.title}</div>
        <div class="muted">{t("playlistCount", { selected: selectedRows.length, total: rows.length })}</div>
      </div>
      <label class="shared">
        <span class="muted">{t("allFormats")}</span>
        <select bind:value={sharedFormat} onchange={applySharedFormat}>
          {#each FORMATS as format (format)}
            <option value={format}>{formatName(format)}</option>
          {/each}
        </select>
      </label>
    </div>

    <label class="select-all">
      <input type="checkbox" checked={allSelected} onchange={toggleAll} />
      <span>{t("selectAll")}</span>
    </label>

    <ul class="rows">
      {#each rows as row, index (index)}
        <li class="row">
          <input type="checkbox" aria-label={row.entry.title} bind:checked={row.selected} />
          <div class="name truncate" class:muted={!row.selected} title={row.entry.title}>
            {index + 1}. {row.entry.title}
          </div>
          {#if row.entry.duration !== null}
            <span class="muted">{formatDuration(row.entry.duration)}</span>
          {/if}
          <select
            class="format"
            aria-label={t("format")}
            value={row.format}
            onchange={(event) => setFormat(row, event.currentTarget.value as Format)}
          >
            {#each FORMATS as format (format)}
              <option value={format}>{formatName(format)}</option>
            {/each}
          </select>
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

    <div class="dialog-actions">
      <button type="button" class="button" onclick={onclose}>{t("cancel")}</button>
      <button type="button" class="button primary" disabled={selectedRows.length === 0} onclick={addSelected}>
        {t("addSelected", { count: selectedRows.length })}
      </button>
    </div>
  </div>
</Dialog>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .summary {
    flex: 1;
    min-width: 0;
  }

  .shared {
    display: flex;
    flex: none;
    align-items: center;
    gap: 8px;
  }

  .select-all {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    align-self: flex-start;
    cursor: pointer;
  }

  .rows {
    max-height: min(360px, calc(100vh - 240px));
    margin: 0;
    padding: 0;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 7px;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 4px 10px;
    border-bottom: 1px solid var(--border);
  }

  .row:last-child {
    border-bottom: 0;
  }

  .name {
    flex: 1;
    min-width: 0;
  }

  .format {
    flex: none;
    width: 82px;
  }

  input {
    flex: none;
    margin: 0;
    accent-color: var(--accent);
  }
</style>
