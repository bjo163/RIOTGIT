#!/usr/bin/env node
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, relative, resolve, sep, join } from "node:path";
import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const temporaryRoot = await mkdtemp(join(tmpdir(), "riotgit-lock-drift-"));
const excluded = new Set([".git", "target", "node_modules", ".next", ".turbo", "backups"]);

async function expectFailure(label, command, args, options, pattern) {
  try {
    await execFileAsync(command, args, {
      ...options,
      maxBuffer: 8 * 1024 * 1024,
      timeout: 90_000
    });
  } catch (error) {
    const output = (error.stdout ?? "") + "\n" + (error.stderr ?? "");
    if (!pattern.test(output)) {
      throw new Error(label + " rejected drift for an unexpected reason:\n" + output.slice(-4000));
    }
    console.log(label + ": stale lockfile correctly rejected the modified manifest.");
    return;
  }
  throw new Error(label + ": command accepted a manifest/lockfile mismatch.");
}

try {
  await cp(root, temporaryRoot, {
    recursive: true,
    filter(source) {
      const path = relative(root, source);
      return path === "" || !path.split(sep).some(part => excluded.has(part));
    }
  });

  const cargoManifestPath = join(temporaryRoot, "apps", "api", "Cargo.toml");
  const cargoManifest = await readFile(cargoManifestPath, "utf8");
  const cargoUpdated = cargoManifest.replace(
    /^version = "0\.2\.0"$/m,
    'version = "0.2.1"'
  );
  if (cargoUpdated === cargoManifest) {
    throw new Error("Could not locate the expected API package version for the Cargo drift fixture.");
  }
  await writeFile(cargoManifestPath, cargoUpdated, "utf8");

  await expectFailure(
    "Cargo --locked",
    "cargo",
    ["metadata", "--format-version", "1", "--manifest-path", join(temporaryRoot, "Cargo.toml"), "--locked"],
    { cwd: temporaryRoot },
    /lock file.*needs to be updated|needs to be updated but.*locked.*was passed/i
  );

  const webManifestPath = join(temporaryRoot, "apps", "web", "package.json");
  const webManifest = JSON.parse(await readFile(webManifestPath, "utf8"));
  webManifest.dependencies = {
    ...webManifest.dependencies,
    "@riotgit/lock-drift-fixture": "0.0.1"
  };
  await writeFile(webManifestPath, JSON.stringify(webManifest, null, 2) + "\n", "utf8");

  await expectFailure(
    "npm ci",
    "npm",
    ["ci", "--ignore-scripts", "--no-audit", "--no-fund"],
    { cwd: join(temporaryRoot, "apps", "web") },
    /package\.json and package-lock\.json are in sync|Missing: .* from lock file|EUSAGE/i
  );
} finally {
  await rm(temporaryRoot, { recursive: true, force: true });
}
