<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { engineStatus, getSettings, setConcurrency, setSaveDir, type EngineStatus, type Settings } from "./api";
  import Dialog from "./Dialog.svelte";
  import { t } from "./i18n";
  import Select from "./Select.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const CONCURRENCY_CHOICES = [1, 2, 3, 4, 5];

  let settings = $state<Settings | null>(null);
  let engine = $state<EngineStatus | null>(null);
  let failure = $state<string | null>(null);

  const engineProblem = $derived(engine?.state === "failed" || engine?.state === "restored");

  onMount(() => {
    getSettings().then((values) => (settings = values));
    engineStatus().then((status) => (engine = status));
    const stop = listen<EngineStatus>("engine", (event) => (engine = event.payload));
    return () => {
      stop.then((unlisten) => unlisten());
    };
  });

  async function apply(action: () => Promise<void>) {
    failure = null;
    try {
      await action();
    } catch (error) {
      failure = String(error);
    }
  }

  function changeConcurrency() {
    const concurrency = settings?.concurrency;
    if (concurrency !== undefined) {
      apply(() => setConcurrency(concurrency));
    }
  }

  async function changeFolder() {
    const saveDir = await openFolderPicker({ directory: true, defaultPath: settings?.saveDir });
    if (typeof saveDir === "string") {
      apply(async () => {
        await setSaveDir(saveDir);
        if (settings) {
          settings.saveDir = saveDir;
        }
      });
    }
  }
</script>

<Dialog label={t("settings")} {onclose}>
  <div class="layout">
    <div class="title">{t("settings")}</div>

    {#if settings}
      <div class="row">
        <span>{t("concurrency")}</span>
        <Select bind:value={settings.concurrency} label={t("concurrency")} width={72} onchange={changeConcurrency}>
          {#each CONCURRENCY_CHOICES as choice (choice)}
            <option value={choice}>{choice}</option>
          {/each}
        </Select>
      </div>

      <div class="row">
        <div class="folder">
          <div>{t("saveFolder")}</div>
          <div class="small muted truncate selectable" title={settings.saveDir}>{settings.saveDir}</div>
        </div>
        <button type="button" class="secondary" onclick={changeFolder}>{t("change")}</button>
      </div>
    {/if}

    {#if failure}
      <div class="small selectable">{failure}</div>
    {/if}
    {#if engine && engineProblem}
      <div class="small selectable" title={engine.detail ?? undefined}>
        {t(engine.state === "failed" ? "engine_failed" : "engine_restored")}
      </div>
    {/if}

    <div class="dialog-actions">
      <button type="button" class="primary" onclick={onclose}>{t("close")}</button>
    </div>
  </div>
</Dialog>

<style>
  .layout {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
  }

  .layout > .title {
    font-size: var(--text-lg);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .folder {
    flex: 1;
    min-width: 0;
  }
</style>
