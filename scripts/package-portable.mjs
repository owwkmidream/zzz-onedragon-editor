import { copyFile, mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const projectDir = path.resolve(scriptDir, "..");
const tauriDir = path.join(projectDir, "src-tauri");
const releaseDir = path.join(tauriDir, "target", "release");
const tauriConfPath = path.join(tauriDir, "tauri.conf.json");
const sourceExePath = path.join(releaseDir, "charge-plan-editor-tauri.exe");
const portableDir = path.join(releaseDir, "bundle", "portable");

function slugify(value) {
  const normalized = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return normalized || "portable-build";
}

const tauriConf = JSON.parse(await readFile(tauriConfPath, "utf8"));
const productSlug = slugify(tauriConf.productName ?? "config-editor");
const version = tauriConf.version ?? "0.0.0";
const portableExeName = `${productSlug}_${version}_x64_portable.exe`;
const portableExePath = path.join(portableDir, portableExeName);

await mkdir(portableDir, { recursive: true });
await copyFile(sourceExePath, portableExePath);

console.log(
  `Portable executable prepared: ${path.relative(projectDir, portableExePath)}`,
);
