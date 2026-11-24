#!/usr/bin/env node
// Enforces the crate dependency rules from AGENTS.md using `cargo metadata`.
// Each crate may depend only on the internal crates listed for it.
import { execFileSync } from "node:child_process";

const ALLOWED = {
  "oxgraft-protocol": [],
  "oxgraft-seams": [],
  "oxgraft-strategies": ["oxgraft-seams"],
  "oxgraft-core": ["oxgraft-seams", "oxgraft-protocol"],
  oxgraft: ["oxgraft-core", "oxgraft-strategies", "oxgraft-seams", "oxgraft-protocol"],
};

let meta;
try {
  meta = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
} catch (err) {
  if (err.code === "ENOENT") {
    console.warn("check-layering: cargo not found; skipped (CI runs it).");
    process.exit(0);
  }
  throw err;
}

const internal = new Set(meta.packages.map((p) => p.name));
const failures = [];
for (const pkg of meta.packages) {
  const allowed = ALLOWED[pkg.name];
  if (!allowed) {
    failures.push(`${pkg.name}: new crate; add its allowed dependencies to scripts/check-layering.mjs`);
    continue;
  }
  for (const dep of pkg.dependencies) {
    if (dep.kind === "dev") continue;
    if (internal.has(dep.name) && !allowed.includes(dep.name)) {
      failures.push(`${pkg.name} must not depend on ${dep.name}`);
    }
  }
}

if (failures.length) {
  console.error("Layering violations:\n  " + failures.join("\n  "));
  process.exit(1);
}
console.log(`check-layering: ${meta.packages.length} crates ok`);
