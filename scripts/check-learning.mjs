#!/usr/bin/env node
// Validates learning/: every watched harness has a profile, every graft
// record has valid frontmatter, and graft origins respect license policy.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { parse } from "smol-toml";

const errors = [];
const watch = parse(readFileSync("learning/watch.toml", "utf8"));
const harnesses = new Map();

const POLICIES = new Set(["adapt", "clean-room", "reference"]);
for (const h of watch.harness ?? []) {
  for (const key of ["id", "name", "repo", "license", "policy"]) {
    if (!h[key]) errors.push(`watch.toml: harness ${h.id ?? "?"} lacks ${key}`);
  }
  if (!POLICIES.has(h.policy)) errors.push(`watch.toml: ${h.id} has unknown policy ${h.policy}`);
  if (harnesses.has(h.id)) errors.push(`watch.toml: duplicate id ${h.id}`);
  harnesses.set(h.id, h);
  const profile = `learning/harnesses/${h.id}.md`;
  if (!existsSync(profile)) errors.push(`${profile} missing (copy learning/harnesses/_TEMPLATE.md)`);
  else {
    const fm = frontmatter(readFileSync(profile, "utf8"));
    if ((fm.last_reviewed ?? "") !== (h.last_reviewed ?? "")) {
      errors.push(`${profile}: last_reviewed "${fm.last_reviewed}" != watch.toml "${h.last_reviewed ?? ""}"`);
    }
  }
}

const STATUSES = new Set(["proposed", "trial", "evaluated", "adopted", "available", "rejected"]);
const KINDS = new Set(["adopt", "avoid"]);
const ORIGINS = new Set(["adapted", "clean-room"]);
const ids = new Set();
for (const file of readdirSync("learning/grafts").filter((f) => /^GRAFT-\d{4}-.+\.md$/.test(f))) {
  const fm = frontmatter(readFileSync(`learning/grafts/${file}`, "utf8"));
  const where = `learning/grafts/${file}`;
  if (!file.startsWith(fm.id + "-")) errors.push(`${where}: id ${fm.id} does not match file name`);
  if (ids.has(fm.id)) errors.push(`${where}: duplicate id ${fm.id}`);
  ids.add(fm.id);
  if (!STATUSES.has(fm.status)) errors.push(`${where}: bad status ${fm.status}`);
  if (!KINDS.has(fm.kind)) errors.push(`${where}: bad kind ${fm.kind}`);
  if (!ORIGINS.has(fm.origin)) errors.push(`${where}: bad origin ${fm.origin}`);
  const donor = harnesses.get(fm.donor);
  if (!donor) errors.push(`${where}: donor ${fm.donor} is not in watch.toml`);
  else if (donor.policy !== "adapt" && fm.origin === "adapted") {
    errors.push(`${where}: donor ${fm.donor} has policy ${donor.policy}; origin must be clean-room`);
  }
  if (["evaluated", "adopted"].includes(fm.status) && !/\n## Evaluation\n[\s\S]*\|\s*with this graft\s*\|\s*\S/.test(readFileSync(where, "utf8"))) {
    errors.push(`${where}: status ${fm.status} requires a filled Evaluation table`);
  }
}

if (errors.length) {
  console.error("Learning records invalid:\n  " + errors.join("\n  "));
  process.exit(1);
}
console.log(`check-learning: ${harnesses.size} harnesses, ${ids.size} grafts ok`);

function frontmatter(text) {
  const m = text.replace(/\r\n/g, "\n").match(/^---\n([\s\S]*?)\n---/);
  if (!m) return {};
  const out = {};
  for (const line of m[1].split("\n")) {
    const kv = line.match(/^([a-z_]+):\s*(.*?)\s*(#.*)?$/);
    if (kv) out[kv[1]] = kv[2].replace(/^"(.*)"$/, "$1");
  }
  return out;
}
