# G1 seeded-review protocol (v1)

G1 answers architecture §4.2: **is the compiled plan a useful human review
surface?** Reviewers see only a `uiko diff` and must decide whether it hides a
consequential change. Runtime stays stubbed; this gate runs before Bet-B
investment.

## Corpus

13 cases in `cases.json`, each an overlay on `./base` (a frozen copy of the
compiler support-console fixture):

- **6 clean decoys** — comment, whitespace and object-key-reorder edits that
  lower to an identical plan. They measure the reviewer false-positive rate.
- **7 seeded cases** — exactly one consequential change each, spanning:
  authorization widening, Managed→Unmanaged execution downgrade,
  contract-fidelity downgrade, component removal, input rebind, options
  narrowing, route change.

Two required §4.2 classes are intentionally absent, with rationale recorded in
issue #39:

- **operation remap** — this fixture has no shape-compatible operation pair, so
  a remap fails type-checking (exit 2) before the diff runs. The
  `query.operation.remapped` dimension is covered by unit tests in `uiko-diff`.
- **invalidation removal** — not a plan/diff dimension; it is Bet-B/M15 runtime
  behavior. Replaced here by the contract-fidelity downgrade.

## Reproduce

```bash
cargo build -p uiko-cli
node experiments/g1/run.mjs --check    # validate corpus vs ground truth
node experiments/g1/run.mjs --packet   # emit anonymized reviewer packet
```

`--check` enforces **mechanical completeness**: every clean case must diff
clean, and every seeded case must surface at least its expected dimension(s).
It is CI-safe (exit 1 on any mismatch).

## Reviewer protocol

- **≥4 reviewers**, including **≥1 who is not implementing uiko**.
- Each reviewer receives only `experiments/g1/packet/R##.diff.txt` — the diff
  text, nothing else. No source, no answer key, no filenames hinting at intent.
- The packet order is a deterministic shuffle, so clean and seeded cases are
  interleaved and the split is not guessable.
- For each `R##`, the reviewer records:
  1. **verdict** — `consequential change` or `no consequential change`;
  2. if consequential, a **one-line description** of what changed;
  3. **intelligibility** — was the diff itself readable, or did its form make the
     change impossible to judge? (`intelligible` / `unintelligible`, with a note).

Answers go in a per-reviewer CSV in `experiments/g1/reviewers/<name>.csv`
(gitignored until the results are frozen). Copy `answer-sheet-template.csv` as
the starting point: `label,verdict,description,intelligibility,note`.

## Scoring

`node experiments/g1/score.mjs` reads `packet/packet-manifest.json`
(label→caseId), `cases.json` and every `reviewers/*.csv`, then reports and
applies the GO/NO-GO rule (exit 0 GO, 1 NO-GO):

- **Per-reviewer detection / false-positive** counts.
- **Per-class detection** — reviewer-observations flagging a seeded class as
  consequential, aggregated by class.
- **Detection point estimate** — hits over all (reviewer × seeded-case) pairs.
- **Unintelligibility** — a class is flagged when marked unintelligible by ≥2
  reviewer-observations (the "repeated" test).

Detection is verdict-based; the free-text `description` is printed for the human
adjudicator but does not gate the estimate. This is why a real human panel — not
a single automated pass — is required: the unintelligibility criterion is
statistical over **independent** reviewers, and §4.2 mandates ≥1 non-implementer.

## GO / NO-GO rule (frozen, §4.2)

GO requires **all three**:

1. **Mechanical completeness** — every seeded change is represented in its diff
   (`run.mjs --check` passes).
2. **Detection point estimate ≥ 70%** across seeded cases.
3. **No repeated semantic-impact class** reported as unintelligible from the diff
   itself.

The decision and per-class results are recorded in an ADR (successor to
`docs/adr/0004`). NO-GO routes findings back into `uiko diff` before Bet-B.
