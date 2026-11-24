// Copies the repo's Markdown (docs/, learning/, top-level guides) into the
// Starlight content folder, adding the `title` frontmatter Starlight needs.
// The repo files stay the source of truth; src/content/docs/{guides,learning,adr}
// are generated and git-ignored.
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const out = resolve(here, "../src/content/docs");

const sections = [
  { dest: "guides", files: ["AGENTS.md", "ROADMAP.md", "STATUS.md", "CONTRIBUTING.md"] },
  { dest: "learning", dir: "learning", files: ["README.md", "capability-matrix.md"] },
  { dest: "learning/harnesses", dir: "learning/harnesses" },
  { dest: "learning/grafts", dir: "learning/grafts" },
  { dest: "adr", dir: "docs/adr" },
];

for (const s of sections) rmSync(join(out, s.dest), { recursive: true, force: true });

let count = 0;
for (const s of sections) {
  const base = s.dir ? join(root, s.dir) : root;
  const names = s.files ?? readdirSync(base).filter((f) => f.endsWith(".md") && !f.startsWith("_") && f !== "TEMPLATE.md");
  for (const name of names) {
    const src = join(base, name);
    if (!existsSync(src)) continue;
    const text = readFileSync(src, "utf8").replace(/\r\n/g, "\n");
    const page = toPage(text, name);
    const slug = name === "README.md" ? "index.md" : name.toLowerCase();
    mkdirSync(join(out, s.dest), { recursive: true });
    writeFileSync(join(out, s.dest, slug), page);
    count++;
  }
}
console.log(`sync-content: ${count} pages`);

function toPage(text, name) {
  let fm = "";
  let body = text;
  const m = text.match(/^---\n([\s\S]*?)\n---\n/);
  if (m) {
    fm = m[1];
    body = text.slice(m[0].length);
  }
  const heading = body.match(/^# (.+)$/m);
  const fmTitle = fm.match(/^title:\s*(.+)$/m);
  const title = (fmTitle?.[1] ?? heading?.[1] ?? basename(name, ".md")).replace(/^"|"$/g, "");
  if (heading) body = body.replace(heading[0], "");
  // Keep the original frontmatter visible as a table-friendly block for grafts and profiles.
  const meta = m ? "\n```yaml\n" + fm + "\n```\n" : "";
  const desc = fm.match(/^status:\s*(\S+)/m)?.[1];
  return `---\ntitle: ${JSON.stringify(title)}\n${desc ? `description: ${JSON.stringify("Status: " + desc)}\n` : ""}---\n${meta}${body}`;
}
