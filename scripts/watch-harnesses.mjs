#!/usr/bin/env node
// Reports which watched harnesses have upstream releases newer than
// last_reviewed in learning/watch.toml. Markdown to stdout (or --json).
// Uses GITHUB_TOKEN (or GH_TOKEN) if set, to avoid API rate limits.
import { readFileSync } from "node:fs";
import { parse } from "smol-toml";

const json = process.argv.includes("--json");
const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
const headers = { Accept: "application/vnd.github+json", "User-Agent": "oxgraft-harness-watch" };
if (token) headers.Authorization = `Bearer ${token}`;

async function gh(path) {
  const res = await fetch(`https://api.github.com/${path}`, { headers });
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`${path}: HTTP ${res.status}`);
  return res.json();
}

async function inspect(h) {
  const repo = await gh(`repos/${h.repo}`);
  if (!repo) return { ...h, error: "repository not found (moved or renamed?)" };
  const release = await gh(`repos/${h.repo}/releases/latest`);
  let latest = release?.tag_name;
  let url = release?.html_url;
  if (!latest) {
    const tags = await gh(`repos/${h.repo}/tags?per_page=1`);
    latest = tags?.[0]?.name ?? `${repo.default_branch}@${repo.pushed_at?.slice(0, 10)}`;
    url = `https://github.com/${h.repo}/tags`;
  }
  const license = repo.license?.spdx_id ?? "none";
  return {
    id: h.id,
    repo: h.repo,
    last_reviewed: h.last_reviewed ?? "",
    latest,
    url,
    stale: (h.last_reviewed ?? "") !== latest,
    archived: repo.archived,
    stars: repo.stargazers_count,
    license_changed: h.policy !== "reference" && license !== h.license ? `${h.license} → ${license}` : null,
    moved: repo.full_name.toLowerCase() !== h.repo.toLowerCase() ? repo.full_name : null,
  };
}

const watch = parse(readFileSync("learning/watch.toml", "utf8"));
const results = [];
for (const h of watch.harness) {
  try {
    results.push(await inspect(h));
  } catch (err) {
    results.push({ id: h.id, repo: h.repo, error: err.message });
  }
}

if (json) {
  console.log(JSON.stringify(results, null, 2));
} else {
  const stale = results.filter((r) => r.stale);
  console.log(`# Harness watch (${new Date().toISOString().slice(0, 10)})\n`);
  console.log(`${stale.length} of ${results.length} harnesses have changes since last review.\n`);
  console.log("| Harness | Last reviewed | Latest | Stars | Flags |\n|---|---|---|---|---|");
  for (const r of results) {
    const flags = [
      r.error && `⚠️ ${r.error}`,
      r.archived && "archived",
      r.moved && `moved to ${r.moved}`,
      r.license_changed && `license changed: ${r.license_changed}`,
    ].filter(Boolean);
    const latest = r.url ? `[${r.latest}](${r.url})` : (r.latest ?? "?");
    const mark = r.stale ? "🔄 " : "";
    console.log(`| ${mark}${r.id} | ${r.last_reviewed || "never"} | ${latest} | ${r.stars ?? "?"} | ${flags.join("; ")} |`);
  }
  console.log("\nTriage with the `watch-harnesses` skill (.agents/skills/watch-harnesses/SKILL.md).");
}

if (results.some((r) => r.license_changed || r.moved || r.error)) process.exitCode = 2;
