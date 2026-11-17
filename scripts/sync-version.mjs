// Numéro de version : package.json est la seule source.
// - tauri.conf.json lit directement package.json ("version": "../package.json").
// - Ce script recopie la version dans src-tauri/Cargo.toml et Cargo.lock.
//
//   node scripts/sync-version.mjs            recopie la version
//   node scripts/sync-version.mjs --check    échoue si les fichiers divergent
//   node scripts/sync-version.mjs --check v1.2.0   vérifie aussi un tag Git
//
// `npm version 1.2.0` l'appelle tout seul (script "version" de package.json), puis crée le commit et le tag v1.2.0.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");
const version = JSON.parse(read("package.json")).version;

const cargoToml = read("src-tauri/Cargo.toml");
const cargoLock = read("src-tauri/Cargo.lock");
const conf = JSON.parse(read("src-tauri/tauri.conf.json"));

const tomlRe = /^(\[package\][\s\S]*?^version = ")([^"]+)(")/m;
const lockRe = /(name = "game-launcher"\r?\nversion = ")([^"]+)(")/;
const current = { toml: cargoToml.match(tomlRe)?.[2], lock: cargoLock.match(lockRe)?.[2] };

const args = process.argv.slice(2);
if (args[0] === "--check") {
  const problems = [];
  if (current.toml !== version) problems.push(`Cargo.toml : ${current.toml} (package.json : ${version})`);
  if (current.lock !== version) problems.push(`Cargo.lock : ${current.lock} (package.json : ${version})`);
  if (conf.version !== "../package.json") problems.push(`tauri.conf.json doit lire la version dans package.json, pas « ${conf.version} »`);
  const tag = args[1]?.replace(/^refs\/tags\//, "");
  if (tag && tag !== `v${version}`) problems.push(`tag ${tag} différent de v${version}`);
  if (problems.length) {
    console.error("Versions désynchronisées :\n- " + problems.join("\n- ") + "\nCorrigez avec : node scripts/sync-version.mjs");
    process.exit(1);
  }
  console.log(`Version ${version} : package.json, Cargo.toml, Cargo.lock et tauri.conf.json sont synchronisés.`);
} else {
  if (!current.toml || !current.lock) throw new Error("version introuvable dans Cargo.toml ou Cargo.lock");
  writeFileSync(join(root, "src-tauri/Cargo.toml"), cargoToml.replace(tomlRe, `$1${version}$3`));
  writeFileSync(join(root, "src-tauri/Cargo.lock"), cargoLock.replace(lockRe, `$1${version}$3`));
  console.log(`Version ${version} recopiée dans Cargo.toml et Cargo.lock.`);
}
