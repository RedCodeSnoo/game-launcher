// Notes de version d'une version, lues dans CHANGELOG.md (section « ## [1.2.0] … » jusqu'à la suivante).
//   node scripts/release-notes.mjs 1.2.0     (ou v1.2.0)
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const version = (process.argv[2] ?? "").replace(/^refs\/tags\//, "").replace(/^v/, "");
const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8").replace(/\r\n/g, "\n");
const escaped = version.replace(/\./g, "\\.");
const match = changelog.match(new RegExp(`^## \\[?${escaped}\\]?[^\\n]*\\n([\\s\\S]*?)(?=^## |(?![\\s\\S]))`, "m"));
const notes = match?.[1].trim();
process.stdout.write(notes || `Version ${version}. Voir CHANGELOG.md pour le détail.`);
