# Code Gene Implementation Plan

## Gene Information
| Field | Value |
|-------|-------|
| **ID** | `67f0d6ff-2b21-4161-b99e-f5734dc659c8` |
| **Crate** | `rairos-rankers-base` |
| **Gap Type** | `evaluation` |
| **Score** | 0.70 |

## Source Paper
- **ID:** `16dd4455-6ba8-4cc2-bbc0-8117747f28f7`
- **Title:** Research Gap: 

## Code Snippet
```rust
// Use `proptest` to generate random queries and document sets, then verify ranker invariants.
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating a list of documents with distinct scores
    fn doc_list() -> impl Strategy<Value = Vec<ScoredDocument>> {
        prop::collection::vec(
            (0..100u64, proptest::num::f64::POSITIVE), // doc_id, positive score
            1..50,
        )
        .prop_map(|v| {
            let mut docs: Vec<ScoredDocument> = v
                .into_iter()
                .map(|(id, score)| ScoredDocument { doc_id: id, score })
                .collect();
            docs.dedup_by_key(|d| d.doc_id);
            docs
        })
    }

    proptest! {
        #[test]
        fn ranked_docs_have_decreasing_scores(docs in doc_list()) {
            // Simulate ranking by sorting descending by score
            let mut ranked = docs.clone();
            ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

            // Verify monotonicity: each successive score must not be greater than the previous
            for window in ranked.windows(2) {
                prop_assert!(window[0].score >= window[1].score,
                    "Scores not decreasing: {:?} > {:?}", window[1].score, window[0].score);
            }
        }

        #[test]
        fn all_docs_appear_in_output(docs in doc_list()) {
            let mut ranked = docs.clone();
            ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

            let mut input_ids: Vec<u64> = docs.iter().map(|d| d.doc_id).collect();
            input_ids.sort();
            let mut output_ids: Vec<u64> = ranked.iter().map(|d| d.doc_id).collect();
            output_ids.sort();
            prop_assert_eq!(input_ids, output_ids, "Ranked output is missing some documents");
        }
    }
}
```
## Existing Related Files
- `crates/rairos-slides/src/lib.rs`
- `crates/rairos-llm/src/gap_detector.rs`
- `crates/rairos-mcp/src/handlers/synthesis/whatif_oracle.rs`
- `crates/rairos-mcp/src/llm_handlers/gap.rs`
- `crates/rairos-topic-discovery/src/lib.rs`

## Implementation Checklist
- [ ] Code compiles
- [ ] Tests pass
- [ ] No duplication
- [ ] Review approved
