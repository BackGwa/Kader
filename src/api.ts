import { invoke } from "@tauri-apps/api/core";

export type Format = "mp4" | "webm" | "mkv" | "m4a" | "opus" | "mp3";

export type Quality = {
  value: number | null;
  codec: string | null;
  size: number | null;
};

export type QualityOptions = Partial<Record<Format, Quality[]>>;

export type VideoInfo = {
  kind: "video";
  url: string;
  title: string;
  duration: number | null;
  thumbnail: string | null;
  options: QualityOptions;
};

export type PlaylistEntry = {
  url: string;
  title: string;
  duration: number | null;
  thumbnail: string | null;
};

export type PlaylistInfo = {
  kind: "playlist";
  title: string;
  entries: PlaylistEntry[];
};

export type Probe = VideoInfo | PlaylistInfo;

export type Selection = {
  format: Format;
  quality: number | null;
};

export type NewJob = {
  url: string;
  title: string;
  thumbnail: string | null;
  selection: Selection;
  label: string;
};

export type JobStatus =
  | "queued"
  | "fetching"
  | "downloading"
  | "processing"
  | "cancelling"
  | "done"
  | "failed"
  | "cancelled";

export type Job = {
  id: number;
  url: string;
  title: string;
  thumbnail: string | null;
  selection: Selection;
  label: string;
  status: JobStatus;
  progress: number;
  speed: number | null;
  size: number | null;
  error: string | null;
};

export type Settings = {
  concurrency: number;
  saveDir: string;
};

export type EngineStatus = {
  state: "checking" | "updating" | "ready" | "failed" | "restored";
  detail: string | null;
};

let nextToken = 1;

export const VIDEO_FORMATS: Format[] = ["mp4", "webm", "mkv"];
export const AUDIO_FORMATS: Format[] = ["m4a", "opus", "mp3"];

export const newToken = () => nextToken++;
export const probe = (token: number, url: string) => invoke<Probe>("probe", { token, url });
export const cancelProbe = (token: number) => invoke<void>("cancel_probe", { token });
export const enqueue = (items: NewJob[]) => invoke<void>("enqueue", { items });
export const cancelJob = (id: number) => invoke<void>("cancel", { id });
export const retryJob = (id: number) => invoke<void>("retry", { id });
export const removeJob = (id: number) => invoke<void>("remove", { id });
export const revealJob = (id: number) => invoke<void>("reveal", { id });
export const snapshot = () => invoke<Job[]>("snapshot");
export const getSettings = () => invoke<Settings>("get_settings");
export const setConcurrency = (concurrency: number) => invoke<void>("set_concurrency", { concurrency });
export const setSaveDir = (saveDir: string) => invoke<void>("set_save_dir", { saveDir });
export const engineStatus = () => invoke<EngineStatus>("engine_status");
export const confirmQuit = () => invoke<void>("confirm_quit");
