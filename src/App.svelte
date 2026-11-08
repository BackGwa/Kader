<script lang="ts">
  import { Link, LoaderCircle, Plus, Settings as SettingsIcon, X } from "@lucide/svelte";
  import { listen } from "@tauri-apps/api/event";
  import { readText } from "@tauri-apps/plugin-clipboard-manager";
  import { onMount } from "svelte";
  import AddDialog from "./AddDialog.svelte";
  import { cancelProbe, confirmQuit, newToken, probe, type PlaylistInfo, type VideoInfo } from "./api";
  import Dialog from "./Dialog.svelte";
  import JobRow from "./JobRow.svelte";
  import { jobs } from "./jobs.svelte";
  import { t } from "./i18n";
  import PlaylistPicker from "./PlaylistPicker.svelte";
  import Settings from "./Settings.svelte";

  type Modal =
    | { kind: "none" }
    | { kind: "analyzing"; token: number }
    | { kind: "video"; info: VideoInfo }
    | { kind: "playlist"; info: PlaylistInfo }
    | { kind: "settings" }
    | { kind: "quit"; count: number };

  const NO_MODAL: Modal = { kind: "none" };
  const SHORTCUT = navigator.platform.startsWith("Mac") ? "⌘V" : "Ctrl+V";

  let modal = $state<Modal>(NO_MODAL);
  let notice = $state<string | null>(null);

  onMount(() => {
    const stops = [
      jobs.connect(),
      listen<number>("quit-requested", (event) => {
        modal = { kind: "quit", count: event.payload };
      }),
    ];
    return () => {
      stops.forEach((stop) => stop.then((unlisten) => unlisten()));
    };
  });

  function linkFrom(text: string) {
    try {
      const url = new URL(text.trim());
      return url.protocol === "http:" || url.protocol === "https:" ? url.href : null;
    } catch {
      return null;
    }
  }

  async function clipboardLink() {
    try {
      return linkFrom(await readText());
    } catch {
      return null;
    }
  }

  async function pasteLink() {
    notice = null;
    const url = await clipboardLink();
    if (!url) {
      notice = t("noLink");
      return;
    }
    const token = newToken();
    modal = { kind: "analyzing", token };
    const isCurrent = () => modal.kind === "analyzing" && modal.token === token;
    try {
      const info = await probe(token, url);
      if (!isCurrent()) {
        return;
      }
      if (info.kind === "video") {
        modal = { kind: "video", info };
      } else if (info.entries.length > 0) {
        modal = { kind: "playlist", info };
      } else {
        modal = NO_MODAL;
        notice = t("emptyPlaylist");
      }
    } catch (error) {
      if (isCurrent()) {
        modal = NO_MODAL;
        notice = String(error);
      }
    }
  }

  function cancelAnalysis() {
    if (modal.kind === "analyzing") {
      cancelProbe(modal.token);
      modal = NO_MODAL;
    }
  }

  function closeModal() {
    modal = NO_MODAL;
  }

  function handleKeydown(event: KeyboardEvent) {
    const isPaste = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "v";
    if (isPaste && modal.kind === "none") {
      event.preventDefault();
      pasteLink();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main>
  <header>
    <button class="button primary" onclick={pasteLink}>
      <Plus size={16} aria-hidden="true" />
      {t("pasteLink")}
    </button>
    <button class="icon-button" title={t("settings")} aria-label={t("settings")} onclick={() => (modal = { kind: "settings" })}>
      <SettingsIcon size={18} aria-hidden="true" />
    </button>
  </header>

  {#if notice}
    <div class="notice" role="status">
      <span class="selectable">{notice}</span>
      <button class="icon-button" aria-label={t("close")} onclick={() => (notice = null)}>
        <X size={16} aria-hidden="true" />
      </button>
    </div>
  {/if}

  {#if jobs.list.length > 0}
    <ul>
      {#each jobs.list as job (job.id)}
        <JobRow {job} />
      {/each}
    </ul>
  {:else}
    <div class="empty">
      <Link size={32} aria-hidden="true" />
      <div class="title">{t("emptyTitle")}</div>
      <div class="muted">{t("emptyHint", { shortcut: SHORTCUT })}</div>
    </div>
  {/if}
</main>

{#if modal.kind === "analyzing"}
  <Dialog label={t("analyzing")} onclose={cancelAnalysis}>
    <div class="dialog-body">
      <div class="analyzing" role="status">
        <LoaderCircle class="spin" size={18} aria-hidden="true" />
        {t("analyzing")}
      </div>
      <div class="dialog-actions">
        <button class="button" onclick={cancelAnalysis}>{t("cancel")}</button>
      </div>
    </div>
  </Dialog>
{:else if modal.kind === "video"}
  <AddDialog info={modal.info} onclose={closeModal} />
{:else if modal.kind === "playlist"}
  <PlaylistPicker info={modal.info} onclose={closeModal} />
{:else if modal.kind === "settings"}
  <Settings onclose={closeModal} />
{:else if modal.kind === "quit"}
  <Dialog label={t("quitTitle")} onclose={closeModal}>
    <div class="dialog-body">
      <div class="title">{t("quitTitle")}</div>
      <div>{t("quitBody", { count: modal.count })}</div>
      <div class="dialog-actions">
        <button class="button" onclick={closeModal}>{t("quitKeep")}</button>
        <button class="button primary" onclick={confirmQuit}>{t("quitConfirm")}</button>
      </div>
    </div>
  </Dialog>
{/if}

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  header {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }

  .notice {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 8px 4px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  ul {
    flex: 1;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }

  .empty {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--text-muted);
  }

  .empty .title {
    color: var(--text);
    margin-top: 8px;
  }

  .analyzing {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
  }

  .analyzing :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
