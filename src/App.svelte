<script lang="ts">
  import { Settings as SettingsIcon } from "@lucide/svelte";
  import { listen } from "@tauri-apps/api/event";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { cancelProbe, confirmQuit, newToken, probe, type PlaylistInfo, type VideoInfo } from "./api";
  import { linkFrom } from "./format";
  import { t } from "./i18n";
  import JobRow from "./JobRow.svelte";
  import { jobs } from "./jobs.svelte";
  import LinkField from "./LinkField.svelte";
  import PlaylistDialog from "./PlaylistDialog.svelte";
  import Settings from "./Settings.svelte";
  import VideoDialog from "./VideoDialog.svelte";

  type Stage =
    | { kind: "idle" }
    | { kind: "analyzing"; token: number }
    | { kind: "video"; info: VideoInfo }
    | { kind: "playlist"; info: PlaylistInfo };

  const IDLE: Stage = { kind: "idle" };

  let stage = $state<Stage>(IDLE);
  let link = $state("");
  let notice = $state<string | null>(null);
  let settingsOpen = $state(false);
  let field: LinkField;
  let quitPromptOpen = false;

  onMount(() => {
    field.focus();
    const stops = [jobs.connect(), listen<number>("quit-requested", (event) => promptQuit(event.payload))];
    return () => {
      stops.forEach((stop) => stop.then((unlisten) => unlisten()));
    };
  });

  async function promptQuit(count: number) {
    if (quitPromptOpen) {
      return;
    }
    quitPromptOpen = true;
    const confirmed = await ask(t("quitBody", { count }), {
      title: t("quitTitle"),
      kind: "warning",
      okLabel: t("quitConfirm"),
      cancelLabel: t("quitKeep"),
    });
    quitPromptOpen = false;
    if (confirmed) {
      confirmQuit();
    }
  }

  function stopAnalysis() {
    if (stage.kind === "analyzing") {
      cancelProbe(stage.token);
    }
  }

  async function analyze(text: string) {
    const url = linkFrom(text);
    if (!url) {
      notice = t("notLink");
      return;
    }
    stopAnalysis();
    notice = null;
    const token = newToken();
    stage = { kind: "analyzing", token };
    const isCurrent = () => stage.kind === "analyzing" && stage.token === token;
    try {
      const info = await probe(token, url);
      if (!isCurrent()) {
        return;
      }
      if (info.kind === "video") {
        stage = { kind: "video", info };
      } else if (info.entries.length > 0) {
        stage = { kind: "playlist", info };
      } else {
        stage = IDLE;
        notice = t("emptyPlaylist");
      }
    } catch (error) {
      if (isCurrent()) {
        stage = IDLE;
        notice = String(error);
      }
    }
  }

  function reset() {
    stopAnalysis();
    stage = IDLE;
    link = "";
    notice = null;
    field.focus();
  }

  function handleContextMenu(event: MouseEvent) {
    const target = event.target;
    if (!(target instanceof Element && target.closest('input[type="text"], .selectable'))) {
      event.preventDefault();
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    const typing = event.target instanceof HTMLInputElement && event.target.type === "text";
    const dialogOpen = stage.kind === "video" || stage.kind === "playlist" || settingsOpen;
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "v" && !typing && !dialogOpen) {
      field.focus();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} oncontextmenu={handleContextMenu} />

<main>
  <header>
    <LinkField bind:this={field} bind:value={link} busy={stage.kind === "analyzing"} onanalyze={analyze} onclear={reset} />
    <button class="icon-button" title={t("settings")} aria-label={t("settings")} onclick={() => (settingsOpen = true)}>
      <SettingsIcon size={16} aria-hidden="true" />
    </button>
  </header>

  {#if notice}
    <p class="small selectable" role="status">{notice}</p>
  {/if}

  <ul>
    {#each jobs.list as job (job.id)}
      <JobRow {job} onfailure={(message) => (notice = message)} />
    {/each}
  </ul>
</main>

{#if stage.kind === "video"}
  {#key stage.info}
    <VideoDialog info={stage.info} onclose={reset} />
  {/key}
{:else if stage.kind === "playlist"}
  {#key stage.info}
    <PlaylistDialog info={stage.info} onclose={reset} />
  {/key}
{/if}

{#if settingsOpen}
  <Settings onclose={() => (settingsOpen = false)} />
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
    gap: var(--space-2);
    height: 56px;
    padding: 0 var(--space-5);
  }

  p {
    flex: none;
    margin: 0;
    padding: 0 var(--space-5) var(--space-3);
  }

  ul {
    flex: 1;
    min-height: 0;
    padding: 0 var(--space-5);
    overflow-y: auto;
  }
</style>
