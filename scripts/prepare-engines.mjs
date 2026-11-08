import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readSync,
  closeSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const YT_DLP_VERSION = "2026.10.26";
const DENO_VERSION = "v2.9.8";
const FFMPEG_LICENSE_VERSION = "n9.0.2";
const UNIVERSAL_BINARY_MAGIC = "cafebabe";

const TARGETS = {
  "darwin-arm64": {
    directory: "macos-arm64",
    ytDlp: {
      asset: "yt-dlp_macos.zip",
      sha256: "20d98fa608998eff87d1d35a6e9fb65e4b874639a6cc89addc19da3becde3ef1",
    },
    deno: {
      asset: "deno-aarch64-apple-darwin.zip",
      sha256: "9f0bf660393da2eaff044cdc7444cb08c73f235849b61d224d825a194ce7c98e",
    },
    ffmpeg: {
      url: "https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip",
      sha256: "c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924",
      root: ".",
    },
  },
  "win32-x64": {
    directory: "windows-x64",
    ytDlp: {
      asset: "yt-dlp_win.zip",
      sha256: "fd51b212cf8380c7a52ee08c0d6be7f3c59b2f2c832eaf7b45acfae6eab57419",
    },
    deno: {
      asset: "deno-x86_64-pc-windows-msvc.zip",
      sha256: "cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf",
    },
    ffmpeg: {
      url: "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/autobuild-2026-11-07-23-13/ffmpeg-N-128606-gcaa98b4938-win64-gpl-shared.zip",
      sha256: "52d99d1a011835ecfac50a16255a635880ca42c7cc79aab418e09ac7286ae084",
      root: "ffmpeg-N-128606-gcaa98b4938-win64-gpl-shared/bin",
    },
  },
};

const target = TARGETS[`${process.platform}-${process.arch}`];
if (!target) {
  throw new Error(`Kader does not support ${process.platform}-${process.arch}`);
}

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const engines = join(projectRoot, "src-tauri", "engines", target.directory);
const stampPath = join(engines, "engines.lock");
const stamp = [target.ytDlp.sha256, target.deno.sha256, target.ffmpeg.sha256].join("\n");

if (!existsSync(stampPath) || readFileSync(stampPath, "utf8") !== stamp) {
  await prepare();
}

async function prepare() {
  rmSync(engines, { recursive: true, force: true });
  mkdirSync(engines, { recursive: true });
  const workspace = mkdtempSync(join(dirname(engines), ".download-"));
  try {
    await installYtDlp(workspace);
    await installDeno(workspace);
    await installFfmpeg(workspace);
    await installLicenses();
    writeFileSync(stampPath, stamp);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
}

async function installYtDlp(workspace) {
  const { asset, sha256 } = target.ytDlp;
  const url = `https://github.com/yt-dlp/yt-dlp/releases/download/${YT_DLP_VERSION}/${asset}`;
  const destination = join(engines, "yt-dlp");
  await downloadAndExtract(url, sha256, workspace, destination);
  if (process.platform === "darwin") {
    keepNativeArchitecture(destination);
  }
  writeFileSync(join(engines, "yt-dlp.sha256"), sha256);
}

async function installDeno(workspace) {
  const { asset, sha256 } = target.deno;
  const url = `https://github.com/denoland/deno/releases/download/${DENO_VERSION}/${asset}`;
  await downloadAndExtract(url, sha256, workspace, engines);
}

async function installFfmpeg(workspace) {
  const { url, sha256, root } = target.ffmpeg;
  const extracted = join(workspace, "ffmpeg");
  await downloadAndExtract(url, sha256, workspace, extracted);
  const destination = join(engines, "ffmpeg");
  mkdirSync(destination);
  const source = join(extracted, root);
  for (const name of readdirSync(source)) {
    if (name === "ffmpeg" || name === "ffmpeg.exe" || name.endsWith(".dll")) {
      cpSync(join(source, name), join(destination, name));
    }
  }
}

async function installLicenses() {
  const directory = join(engines, "licenses");
  mkdirSync(directory);
  const documents = {
    "FFmpeg-GPLv3.txt": `https://raw.githubusercontent.com/FFmpeg/FFmpeg/${FFMPEG_LICENSE_VERSION}/COPYING.GPLv3`,
    "Deno-MIT.md": `https://raw.githubusercontent.com/denoland/deno/${DENO_VERSION}/LICENSE.md`,
  };
  for (const [name, url] of Object.entries(documents)) {
    writeFileSync(join(directory, name), await fetchBytes(url));
  }
  cpSync(
    join(engines, "yt-dlp", "_internal", "THIRD_PARTY_LICENSES.txt"),
    join(directory, "yt-dlp-THIRD_PARTY_LICENSES.txt"),
  );
}

async function downloadAndExtract(url, sha256, workspace, destination) {
  const bytes = await fetchBytes(url);
  const actual = createHash("sha256").update(bytes).digest("hex");
  if (actual !== sha256) {
    throw new Error(`Checksum mismatch for ${url}`);
  }
  const archive = join(workspace, `${sha256}.zip`);
  writeFileSync(archive, bytes);
  mkdirSync(destination, { recursive: true });
  execFileSync(archiveTool(), ["-xf", archive, "-C", destination]);
}

async function fetchBytes(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to download ${url}: ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

function archiveTool() {
  return process.platform === "win32"
    ? join(process.env.SystemRoot, "System32", "tar.exe")
    : "/usr/bin/tar";
}

function keepNativeArchitecture(directory) {
  for (const name of readdirSync(directory)) {
    const path = join(directory, name);
    if (statSync(path).isDirectory()) {
      keepNativeArchitecture(path);
    } else if (isUniversalBinary(path)) {
      const thinned = `${path}.thin`;
      const mode = statSync(path).mode;
      execFileSync("lipo", ["-thin", "arm64", path, "-output", thinned]);
      renameSync(thinned, path);
      chmodSync(path, mode);
    }
  }
}

function isUniversalBinary(path) {
  const descriptor = openSync(path, "r");
  const header = Buffer.alloc(4);
  const length = readSync(descriptor, header, 0, 4, 0);
  closeSync(descriptor);
  return length === 4 && header.toString("hex") === UNIVERSAL_BINARY_MAGIC;
}
