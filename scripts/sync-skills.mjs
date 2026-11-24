#!/usr/bin/env node
// .agents/skills/ is the source of truth (read by Codex, Cursor, Gemini CLI,
// Copilot, Antigravity). Claude Code reads .claude/skills/, so mirror it there.
//   node scripts/sync-skills.mjs          write the mirror
//   node scripts/sync-skills.mjs --check  fail if the mirror is stale
import { cpSync, existsSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const SRC = ".agents/skills";
const DEST = ".claude/skills";
const check = process.argv.includes("--check");

function files(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    return statSync(p).isDirectory() ? files(p) : [p];
  });
}

const normalize = (s) => s.replace(/\r\n/g, "\n");
const src = new Map(files(SRC).map((f) => [relative(SRC, f), normalize(readFileSync(f, "utf8"))]));
const dest = new Map(files(DEST).map((f) => [relative(DEST, f), normalize(readFileSync(f, "utf8"))]));

for (const [rel, text] of src) {
  if (!/^---\nname: [a-z0-9-]+\ndescription: .+\n/m.test(text) && rel.endsWith("SKILL.md")) {
    console.error(`${SRC}/${rel}: needs frontmatter with name and description`);
    process.exit(1);
  }
}

const stale = [
  ...[...src].filter(([rel, text]) => dest.get(rel) !== text).map(([rel]) => rel),
  ...[...dest.keys()].filter((rel) => !src.has(rel)),
];

if (check) {
  if (stale.length) {
    console.error(`Skills mirror is stale (${stale.join(", ")}). Run: pnpm skills:sync`);
    process.exit(1);
  }
  console.log(`sync-skills: ${src.size} files in sync`);
} else {
  rmSync(DEST, { recursive: true, force: true });
  cpSync(SRC, DEST, { recursive: true });
  console.log(`sync-skills: mirrored ${src.size} files to ${DEST}`);
}
