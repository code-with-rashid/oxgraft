use oxgraft_seams::{ContextItem, ContextStrategy, Provenance, Seam, Strategy, StrategyInfo};

/// Keep every pinned item, then add the rest in order while they fit.
///
/// Deliberately naive baseline. Candidate grafts: Aider-style repo map,
/// OpenCode/Pi-style compaction, retrieval.
#[derive(Debug, Clone, Copy, Default)]
pub struct PinnedThenInOrder;

static INFO: StrategyInfo = StrategyInfo {
    id: "pinned_then_in_order",
    seam: Seam::Context,
    summary: "Pinned items always; the rest in order until the token budget is spent.",
    provenance: Provenance::original("Baseline to measure smarter context strategies against."),
};

impl Strategy for PinnedThenInOrder {
    fn info(&self) -> &StrategyInfo {
        &INFO
    }
}

impl ContextStrategy for PinnedThenInOrder {
    fn select(&self, items: &[ContextItem], budget: usize) -> Vec<String> {
        let mut used: usize = items.iter().filter(|i| i.pinned).map(|i| i.tokens).sum();
        let mut chosen = Vec::new();
        for item in items {
            if item.pinned {
                chosen.push(item.id.clone());
            } else if used + item.tokens <= budget {
                used += item.tokens;
                chosen.push(item.id.clone());
            }
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, tokens: usize, pinned: bool) -> ContextItem {
        ContextItem { id: id.into(), tokens, pinned }
    }

    #[test]
    fn keeps_pinned_and_fills_budget_in_order() {
        let items = [item("sys", 50, true), item("a", 30, false), item("b", 40, false), item("c", 10, false)];
        assert_eq!(PinnedThenInOrder.select(&items, 100), vec!["sys", "a", "c"]);
    }

    #[test]
    fn pinned_items_survive_an_exhausted_budget() {
        let items = [item("sys", 500, true), item("a", 1, false)];
        assert_eq!(PinnedThenInOrder.select(&items, 100), vec!["sys"]);
    }
}
