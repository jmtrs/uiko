# ADR 0005: G1 semantic-diff review GO at pilot grade

- Status: Accepted
- Date: 2026-09-20

## Context

Issue #39 asked whether a compiled-plan `uiko diff` (M13/#38) is a useful
**human review surface** (architecture §4.2), gating Bet-B managed-runtime work.
The corpus, protocol and scoring driver were built in `experiments/g1/`: a frozen
`base/` project, 13 overlay cases (6 clean decoys + 7 seeded, one consequential
change each), `cases.json` ground truth, `run.mjs` (`--check` mechanical
completeness, `--packet`/`--handout` reviewer materials, `--review` interactive
capture) and `score.mjs` (detection, false-positive, per-class, GO/NO-GO).

The seed set was adjusted against the shipped diff's real taxonomy, recorded in
#39: **operation-remap** was dropped (this fixture has no shape-compatible
operation pair, so a remap fails type-checking before the diff runs; the
`query.operation.remapped` dimension is unit-tested in `uiko-diff`), and
**invalidation-removal** was dropped (a Bet-B/M15 runtime concept, not a plan
dimension) in favor of a contract-fidelity downgrade. Covered classes:
authorization widening, Managed→Unmanaged execution downgrade, contract-fidelity
downgrade, component removal, input rebind, options narrowing, route change.

A synthetic-reviewer dry-run flagged `query.output.changed` as unintelligible —
its row printed two ~600-char single-line JSON blobs. This was fixed before the
human run (`uiko-diff` `to_human` now emits a path-addressed field-level delta
for structured rows; the JSON envelope is unchanged).

## Decision

1. **G1 is a GO at pilot grade.** The owner reviewed the corpus from the diffs
   alone (`run.mjs --review`, `reviewers/jose.csv`): **detection 7/7 (100%)**,
   **false-positive 0/6**, mechanical completeness **13/13**, and **no class
   marked unintelligible**. This clears the frozen §4.2 rule (mechanical
   completeness; detection ≥ 70%; no repeated unintelligible class).
2. **The field-level delta rendering ships as part of G1.** It is what makes the
   contract/output class intelligible; it stayed byte-deterministic and left the
   machine JSON envelope untouched.
3. **Bet-B / M15 is unblocked** on the strength of this pilot.

## Consequences

- Proceed to M15 (#40, mutation and managed runtime core).
- This is **pilot grade, not protocol grade**: n=1 reviewer, who is also the
  implementer. §4.2's "≥1 non-implementer reviewer" is **not** satisfied. That
  evidence is deferred to the publication-grade study (M19, §24.11); any external
  claim of a measured G1 result would be false today.
- The clean decoys lower to a literal `uiko diff: clean`, so they test only
  false invention, not over-flagging of real-but-benign changes. A richer
  false-positive axis (benign changes that surface rows) is future work.
- Reviewer answer data lives under `experiments/g1/reviewers/` (gitignored until
  a results set is frozen for publication).
