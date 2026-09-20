# G1 local experiment — semantic-diff review pilot

> **Status (2026-09-20): corpus ready, pilot not yet run.** The corpus,
> protocol and scoring driver are committed and reproducible. Reviewer runs and
> the GO/NO-GO ADR are pending. Tracking: [#39](https://github.com/jmtrs/uiko/issues/39).

G1 is the second product gate ([architecture §4.2](../../docs/architecture/uiko_Product_Technical_Architecture_PoC_v0.16.md)):
does a compiled-plan **`uiko diff`** let a human reviewer catch a consequential
change from the diff alone? It runs on top of `uiko diff` (M13/#38) and before
any Bet-B managed-runtime work.

## Layout

```text
base/            frozen review target (copy of the compiler support-console fixture)
corpus/<id>/     per-case overlay — only the files that differ from base
cases.json       ground truth: id, clean|seeded, class, expected diff kinds
run.mjs          materialize base+overlay, run `uiko diff`, --check / --packet
score.mjs        read reviewer CSVs, compute detection / FP / GO-NO-GO
answer-sheet-template.csv  blank per-reviewer answer sheet (R01..R13)
protocol.md      corpus rationale, reviewer protocol, scoring, GO/NO-GO rule
packet/          generated reviewer packet (gitignored, regenerable)
reviewers/       per-reviewer answer CSVs (gitignored until results frozen)
```

An AFTER project is `base` with a case's overlay files copied over the same
paths; the review surface is `uiko diff base AFTER`.

## Run

```bash
cargo build -p uiko-cli
node experiments/g1/run.mjs            # summary table
node experiments/g1/run.mjs --check    # validate corpus vs ground truth (CI-safe)
node experiments/g1/run.mjs --packet   # emit anonymized packet/R##.diff.txt
node experiments/g1/run.mjs --handout  # one self-contained packet/handout.md to send reviewers
node experiments/g1/score.mjs          # score reviewers/*.csv -> GO/NO-GO
```

To run the panel: `--handout`, send `packet/handout.md` to each reviewer, save
each reply as `reviewers/<name>.csv`, then `score.mjs`.

See [protocol.md](./protocol.md) for the reviewer procedure and the frozen
GO/NO-GO rule.
