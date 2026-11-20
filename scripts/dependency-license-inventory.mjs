#!/usr/bin/env node
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const webRoot = join(root, "apps", "web");
const outputPath = resolve(process.argv[2] ?? join(root, "dependency-license-inventory.tsv"));

function licenseText(value) {
  if (typeof value === "string" && value.trim()) return value.replace(/[\\t\\r\\n]+/g, " ").trim();
  if (value && typeof value === "object" && typeof value.type === "string") return licenseText(value.type);
  return "UNKNOWN";
}

const cargo = await execFileAsync("cargo", ["metadata", "--format-version", "1", "--locked"], {
  cwd: root,
  maxBuffer: 32 * 1024 * 1024
});
const metadata = JSON.parse(cargo.stdout);
const rows = [["ecosystem", "package", "version", "license", "source"].join("\\t")];

for (const pkg of metadata.packages) {
  rows.push(["cargo", pkg.name, pkg.version, licenseText(pkg.license), pkg.source ?? "workspace"].join("\\t"));
}

const lock = JSON.parse(await readFile(join(webRoot, "package-lock.json"), "utf8"));
for (const [location, item] of Object.entries(lock.packages ?? {})) {
  if (!location.includes("node_modules/") || item.link === true) continue;
  let manifest = {};
  try {
    manifest = JSON.parse(await readFile(join(webRoot, location, "package.json"), "utf8"));
  } catch {
    // Optional platform packages may not be installed on this runner.
  }
  const marker = location.lastIndexOf("node_modules/");
  const name = manifest.name ?? location.slice(marker + "node_modules/".length);
  rows.push(["npm", name, item.version ?? manifest.version ?? "UNKNOWN",
    licenseText(manifest.license ?? item.license), "node_modules"].join("\\t"));
}

const dataRows = rows.slice(1).sort((left, right) => left.localeCompare(right));
const unknownCount = dataRows.filter(line => line.split("\\t")[3] === "UNKNOWN").length;
await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${rows[0]}\\n${dataRows.join("\\n")}\\n`, "utf8");
console.log(`Wrote ${dataRows.length} dependency records; ${unknownCount} have unknown license metadata.`);
