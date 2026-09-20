# uiko

**Coding agents write small declarative sources; the uiko compiler turns them into a deterministic application plan humans can review.**

Agents author behavior in `uiko.jsonc` instead of inventing it in
TypeScript. The compiler invents nothing: it resolves, validates and lowers
everything to a plan a renderer executes, failing invalid source before it
reaches the browser. The thesis is one chain:

> **reduce invention → compile behavior → review semantics → govern execution**

Each link is gated by measurement before the next gets investment.

## Status

| Gate | Question | State |
|---|---|---|
| **G0** | Does uiko make an agent author materially less application code? | **GO** — paired pilot 2026-09-18: both arms passed acceptance 4/4, ~3× fewer application tokens ([ADR 0004](docs/adr/0004-g0-pilot-go.md), [evidence](experiments/g0/results/pilot-2026-09-18/README.md)) |
| **G1** | Are semantic plan diffs complete and useful to human reviewers? | **GO (pilot grade)** — solo review 2026-09-20: detection 7/7, 0 false positives, mechanical 13/13 ([ADR 0005](docs/adr/0005-g1-review-go.md), [experiment](experiments/g1/README.md)); non-implementer reviewer deferred to M19 |
| **Bet B** | Managed runtime (authz, audit, mutations) | Next — gate cleared at pilot grade |

Working today: strict JSONC source adapter, pure semantic compiler with
stable diagnostics, `UiManifest` lowering, Vue/json-render host, OpenAPI read
compilation through a logical query gateway, page-local state with
Select/Pagination/Table controls, deterministic G0 experiment core (tasks,
bases, shared Playwright acceptance).

## Pipeline

```text
uiko.jsonc
  -> located source DTOs
  -> resolve / type-check / lower
  -> deterministic AppIr
  -> UiManifest
  -> renderer (Vue host / json-render)
```

Invalid source fails before browser execution with stable machine-readable
diagnostics.

## Repository shape

```text
crates/            uiko-{core,source,source-jsonc,compiler,manifest,
                   runtime-plan,capabilities,openapi,project,cli,
                   g0-gateway,g0-runner}
hosts/             Vue rendering host
fixtures/          support-console acceptance fixture
baselines/         b-full conventional frontend comparison arm
experiments/       G0 task manifests, instrumentation, results
docs/architecture  normative product + technical specification
docs/adr           architecture decision records
```

## Build

Rust 2024 is the semantic implementation baseline.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
```

## Reading order

1. [Architecture specification](docs/architecture/uiko_Product_Technical_Architecture_PoC_v0.16.md) — normative
2. [ADRs](docs/adr/) — decisions to date
3. [G0 experiment](experiments/g0/README.md) — protocol, pilot evidence, results
