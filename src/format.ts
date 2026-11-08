import type { Format } from "./api";
import { t } from "./i18n";

const FORMAT_NAMES: Record<Format, string> = {
  mp4: "MP4",
  webm: "WebM",
  mkv: "MKV",
  m4a: "M4A",
  opus: "Opus",
  mp3: "MP3",
};

const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB"];

export const formatName = (format: Format) => FORMAT_NAMES[format];

export function isVideoFormat(format: Format) {
  return format === "mp4" || format === "webm" || format === "mkv";
}

export function qualityLabel(format: Format, value: number | null) {
  if (value === null) {
    return t("best");
  }
  return isVideoFormat(format) ? `${value}p` : `${value} kbps`;
}

export function formatBytes(bytes: number) {
  const unit = Math.min(Math.floor(Math.log(Math.max(bytes, 1)) / Math.log(1024)), BYTE_UNITS.length - 1);
  const amount = bytes / 1024 ** unit;
  return `${amount.toFixed(unit < 2 ? 0 : 1)} ${BYTE_UNITS[unit]}`;
}

export function formatDuration(seconds: number) {
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remainder = String(total % 60).padStart(2, "0");
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${remainder}`
    : `${minutes}:${remainder}`;
}

export function linkFrom(text: string) {
  try {
    const url = new URL(text.trim());
    return url.protocol === "http:" || url.protocol === "https:" ? url.href : null;
  } catch {
    return null;
  }
}
