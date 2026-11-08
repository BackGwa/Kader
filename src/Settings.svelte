<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { engineStatus, getSettings, setConcurrency, setSaveDir, type EngineStatus, type Settings } from "./api";
  import Dialog from "./Dialog.svelte";
  import { t } from "./i18n";

  let { onclose }: { onclose: () => void } = $props();

  const CONCURRENCY_CHOICES = [1, 2, 3, 4, 5];

  let settings = $state<Settings | null>(null);
  let engine = $state<EngineStatus | null>(null);
  let saveError = $state<string | null>(null);

  onMount(() => {
    getSettings().then((values) => (settings = values));
    engineStatus().then((status) => (engine = status));
    const stop = listen<EngineStatus>("engine", (event) => (engine = event.payload));
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });

  async function save(action: () => Promise<void>) {
    saveError = null;
    try {
      await action();
    } catch (error) {
      saveError = String(error);
    }
  }

  function changeConcurrency(event: Event) {
    const concurrency = Number((event.currentTarget as HTMLSelectElement).value);
    save(async () => {
      await setConcurrency(concurrency);
      if (settings) {
        settings.concurrency = concurrency;
      }
    });
  }

  async function changeFolder() {
    const saveDir = await openFolderPicker({ directory: true, defaultPath: settings?.saveDir });
    if (typeof saveDir === "string") {
      save(async () => {
        await setSaveDir(saveDir);
        if (settings) {
          settings.saveDir = saveDir;
        }
      });
    }
  }
</script>

<Dialog label={t("settings")} {onclose}>
  <div class="dialog-body">
    <div class="title">{t("settings")}</div>

    {#if settings}
      <label class="setting">
        <span>{t("concurrency")}</span>
        <select value={settings.concurrency} onchange={changeConcurrency}>
          {#each CONCURRENCY_CHOICES as choice (choice)}
            <option value={choice}>{choice}</option>
          {/each}
        </select>
      </label>

      <div class="setting">
        <div class="folder">
          <div>{t("saveFolder")}</div>
          <div class="muted truncate" title={settings.saveDir}>{settings.saveDir}</div>
        </div>
        <button type="button" class="button" onclick={changeFolder}>{t("change")}</button>
      </div>
    {/if}

    {#if saveError}
      <div class="muted danger selectable">{saveError}</div>
    {/if}

    {#if engine}
      <div class="setting">
        <span>{t("engine")}</span>
        <span class="muted" class:danger={engine.state === "failed"}>{t(`engine_${engine.state}`)}</span>
      </div>
      {#if engine.detail}
        <div class="muted danger selectable">{engine.detail}</div>
      {/if}
    {/if}

    <div class="dialog-actions">
      <button type="button" class="button" onclick={onclose}>{t("close")}</button>
    </div>
  </div>
</Dialog>

<style>
  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 34px;
  }

  .folder {
    flex: 1;
    min-width: 0;
  }
</style>
