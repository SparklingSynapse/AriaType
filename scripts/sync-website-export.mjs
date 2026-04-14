import { cpSync, existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const sourceDir = path.join(repoRoot, "packages", "website", "out");
const targetDir = path.join(repoRoot, "website");

if (!existsSync(sourceDir)) {
  throw new Error(`Website export directory not found: ${sourceDir}`);
}

mkdirSync(targetDir, { recursive: true });

for (const entry of readdirSync(targetDir)) {
  rmSync(path.join(targetDir, entry), { recursive: true, force: true });
}

cpSync(sourceDir, targetDir, { recursive: true });
writeFileSync(path.join(targetDir, ".nojekyll"), "");

console.log(`Synced website export to ${targetDir}`);
