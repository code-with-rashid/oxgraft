//! Stability: no strategy may panic on any input, however malformed.

use oxgraft_seams::{Action, Command, ContextItem};

/// Small deterministic PRNG (xorshift) so the test needs no dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[(self.next() % items.len() as u64) as usize]
    }
}

const PIECES: &[&str] = &[
    "<<<<<<< SEARCH",
    "=======",
    ">>>>>>> REPLACE",
    "*** Begin Patch",
    "*** Update File: a",
    "*** End Patch",
    "@@",
    "@@ fn a",
    "+added",
    "-removed",
    " context",
    "",
    "fn a() {",
    "}",
    "    indented",
    "\t",
    "é漢字🙂",
    "\r",
    "x = 1",
    "x = 1   ",
    "*** Add File: b",
    "-",
    "+",
    "@@@",
    ">>>>>>>",
    "<<<<<<<",
];

fn random_text(rng: &mut Rng, lines: usize) -> String {
    (0..lines).map(|_| rng.pick(PIECES)).collect::<Vec<_>>().join("\n")
}

#[test]
fn edit_formats_never_panic() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for format in oxgraft_strategies::edit_formats() {
        for _ in 0..3_000 {
            let n = (rng.next() % 12) as usize;
            let original = random_text(&mut rng, n);
            let m = (rng.next() % 16) as usize;
            let edit = random_text(&mut rng, m);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| format.apply(&original, &edit)));
            assert!(result.is_ok(), "{} panicked on original={original:?} edit={edit:?}", format.info().id);
        }
    }
}

#[test]
fn other_seams_never_panic() {
    let mut rng = Rng(42);
    for policy in oxgraft_strategies::approvals() {
        for _ in 0..2_000 {
            let action = Action { tool: rng.pick(PIECES).into(), resource: random_text(&mut rng, 3) };
            let _ = policy.decide(&action);
        }
    }
    for ctx in oxgraft_strategies::contexts() {
        let items: Vec<ContextItem> = (0..50)
            .map(|i| ContextItem {
                id: i.to_string(),
                tokens: (rng.next() % 1_000) as usize,
                pinned: rng.next().is_multiple_of(5),
            })
            .collect();
        for budget in [0, 1, 100, 10_000, usize::MAX] {
            let _ = ctx.select(&items, budget);
        }
    }
    for sandbox in oxgraft_strategies::sandboxes() {
        let _ = sandbox.confinement(&Command { program: "sh".into(), args: vec![random_text(&mut rng, 2)] });
    }
}
