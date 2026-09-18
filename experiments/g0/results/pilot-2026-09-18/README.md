# G0 manual paired pilot — 2026-09-18

Owner decision: **GO**. Recorded in [ADR 0004](../../../../docs/adr/0004-g0-pilot-go.md).

## Question

Before investing in a measured adapter (issue #37), one question dominated
the risk: can a coding model with no UIKO in training learn the jsonc DSL
from repository context alone, author a working feature, and pass the shared
acceptance harness? If not, a measured G0 with this model is pointless.

Secondary: does the authoring-reduction effect predicted by the thesis show
up at all in a paired execution?

## Method

Manual paired pilot, single turn per arm, driven by
`experiments/g0/claude/pilot.mjs` (committed at `b6ba3ff`):

- worktree detached at `b6ba3ff`, clean tree;
- base via `prepareExecutionBase` (B_FULL `b44814a9`, C_UIKO `f110be5d`);
- agent: Claude Code 2.1.198 `--bare -p --output-format stream-json
  --verbose --permission-mode bypassPermissions --model "glm-5.2[1m]"`
  via the z.ai Anthropic-compatible backend (claude-sub-zai), scrubbed env,
  isolated `CLAUDE_CONFIG_DIR`, no tools restriction, no human hints;
- frozen G0-D01 prompt (212 chars) delivered verbatim as the only input;
- acceptance: shared Playwright harness, 4 frozen criteria;
- diff stats against the base revision with the frozen tokenizer
  (`/[\p{L}\p{N}_]+|[^\s]/gu`), application-owned paths only
  (B_FULL `baselines/b-full/src/**` minus `src/api/generated/`;
  C_UIKO `fixtures/support-console/**` minus `integrations/`).

Not measured: M5 trace, `uiko-g0-runner` metrics, repair turns, probes
(R_RENDER_ONLY, T_AUTHORING), tasks D02–D06. This is pilot grade, not the
protocol §9 evaluation.

## Results

Both arms passed acceptance 4/4 on the first agent turn.

| metric | B_FULL | C_UIKO | ratio |
|---|---|---|---|
| acceptance | PASS (4/4) | PASS (4/4) | = |
| application added tokens | 475 | **161** | **2.95× fewer** |
| application added lines | 82 | 26 | 3.2× fewer |
| provider output tokens | 16,571 | 13,767 | 1.2× fewer |
| provider input tokens | 42,437 | 128,065 | 3.0× more |
| cache read tokens | 724,544 | 1,875,584 | 2.6× more |
| agent turns | 35 | 63 | 1.8× more |
| Read tool calls | 1 | 27 | 27× more |
| wall clock | 6.0 min | 5.9 min | = |

Canonical solution ceiling for D01 (frozen prerequisites): 421 vs 177
tokens (2.4×). The agent-level ratio (2.95×) is consistent with — slightly
above — the canonical ceiling.

## Findings

1. **Largest risk eliminated.** glm-5.2 learned the uiko jsonc authoring
   format from repository context (27 Read calls over existing
   `features/**` sources) and produced an accepted feature with no human
   help and no flailing. The DSL is agent-learnable from context.
2. **Authoring reduction is real and measurable.** ~3× fewer application
   tokens and lines written by the agent in C_UIKO, matching the canonical
   ceiling. This is the A1 effect the thesis claims.
3. **The cost structure matches the thesis's own prediction.** C_UIKO
   shifted work from writing to reading: 3× more input tokens, 27× more
   reads, 1.8× more turns, while wall clock stayed equal. Provider token
   totals were *higher* in C_UIKO for this single task. Per the
   architecture, provider tokens are descriptive only; the formal G0
   metric set exists precisely to decompose reads vs edits rather than
   judge net token totals. The pilot does not settle that decomposition.
4. **Caveat for the path policy.** The C_UIKO run left `Cargo.lock +2`
   (classified `other`), from `cargo build` during base preparation. A
   measured G0 should decide whether lockfile churn is excluded or
   attributed.

## Limitations

- n=1 per arm, one task (D01), one model, one session.
- No M5 trace: no read/search/edit event decomposition, no repair data.
- No probes: the "JSON rendering explains it" test (H4) was not run.
- Not the protocol §9 GO/NO-GO evaluation; thresholds there (30% medians
  over six paired tasks) were not applied.

## Artifacts

Per arm (`B_FULL/`, `C_UIKO/`):

- `summary.json` — pilot driver summary (usage, turns, files, tokens);
- `turn-01.prompt.txt` — frozen prompt delivered;
- `turn-01.claude.jsonl.gz` — raw stream-json transcript (gzipped);
- `turn-01.claude.stderr.log` — empty in both runs;
- `acceptance.stdout.log` — shared harness output with per-criterion
  evidence;
- `application.diff` — full diff from the base revision including new
  files.

Session-state directories (`claude-home`) were deliberately not preserved.
