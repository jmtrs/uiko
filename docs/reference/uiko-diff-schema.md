# `uiko diff` — report schema

_Status: implemented for M13 (#38); normative spec surface §16.3, §26.1._

`uiko diff <BEFORE> <AFTER> [--format json]` compiles two project revisions
in-process and reports every semantic change between the two compiled plans
(`AppIr` + capability catalogs). Output is deterministic: identical inputs
produce byte-identical reports.

## Exit codes

Follows diff(1), deliberately different from `uiko validate` (where 1 means
"diagnostics found"):

| Code | Meaning                                        |
| ---- | ---------------------------------------------- |
| 0    | plans are semantically identical (clean)      |
| 1    | plans differ (report printed)                 |
| 2    | usage error, load/compile failure of a side, or unresolved operation reference |

A side that fails to compile emits its diagnostics (same format as
`validate`) and exits 2; no diff is attempted against a broken plan.

## JSON envelope (`--format json`)

```json
{
  "status": "changed",
  "rows": [
    {
      "kind": "query.execution.changed",
      "target": "customers.CustomerList.query.customers",
      "securitySignificant": true,
      "before": { "execution": "managed" },
      "after": { "execution": "unmanaged", "controlsNoLongerClaimed": true }
    }
  ]
}
```

- `status`: `"clean"` (rows empty) or `"changed"`.
- `kind`: stable taxonomy code (below). Rows are sorted by
  `(kind, target, before, after)` and deduplicated.
- `target`: symbolic id — `module`, `{module}.{page}`,
  `{module}.{page}.state.{id}`, `{module}.{page}.query.{alias}`,
  `{query_id}.input.{name}`, `{module}.{page}.{component_id}`,
  `capability.{provider}.{operation}`, or `app` for the two app-level
  dimensions (`app.name.changed`, `app.spec_version.changed`).
- `before` / `after`: canonical compact JSON payloads; absent when the row is
  an addition (no `before`) or removal (no `after`).
- `securitySignificant`: review gates key on this flag (see rules below).

Human format (default) prints one `UIKO-DIFF <kind> <target>
[security-significant]` header per row with indented before/after payloads.

## Taxonomy

| Kind | securitySignificant |
| ---- | ------------------- |
| `app.name.changed` | no |
| `app.spec_version.changed` | no |
| `module.added` / `module.removed` | no |
| `page.added` / `page.removed` | no |
| `page.route.changed` | no |
| `state.added` / `state.removed` / `state.initial.changed` | no |
| `query.added` / `query.removed` | no |
| `query.operation.remapped` | **always** |
| `query.input.added` / `query.input.removed` / `query.input.rebound` | no |
| `query.output.changed` | **always** (contract fidelity) |
| `capability.parameters.changed` | **iff** a before-required parameter is lost or becomes optional |
| `query.execution.changed` | **iff** downgrade (Managed → Unmanaged); after payload carries `controlsNoLongerClaimed: true` (§16.3) |
| `query.authorization.changed` | **iff** widening: `before ∖ after ≠ ∅` |
| `component.added` / `component.removed` | no |
| `component.kind.changed` / `component.field.changed` | no |
| `component.options.changed` | no |
| `component.reordered` | no |

### Widening direction is scope REMOVAL

`query.authorization.changed` is security-significant when a scope present
before is absent after. Removing a required scope widens access (§17.2);
adding scopes only narrows it. This is the opposite of the naive reading and
is pinned by tests in both directions.

### Added/removed queries are not security-flagged

Per the taxonomy, `query.added` / `query.removed` rows never carry the
`securitySignificant` flag — security direction is only defined for changes
to a *surviving* query. A brand-new query that already declares
`execution: "unmanaged"` (a fresh escape path) therefore surfaces as an
unflagged `query.added` row. Review gates that key solely on the flag must
also inspect added queries; whether added-with-`unmanaged` becomes a flagged
condition is an M14 review-semantics decision.

## Ordering semantics

- Modules, pages, state entries, queries, and query inputs are keyed by id:
  reordering them is **not** a change (recompile identity).
- Components and `Select` options are order-semantic (render order):
  `component.reordered` reports a change in the relative order of surviving
  components; insertion or removal among survivors is not a reorder.

## Capability scoping boundary

The diff scopes capabilities to those the app references. Capability
parameters are compared over the referenced-on-both-sides intersection; a
capability referenced by only one side is covered by that side's
`query.added`/`query.removed` rows. Unreferenced catalog churn produces no
row. Capability **output** shape changes surface as `query.output.changed`
rows on the queries referencing the operation (the shape is snapshotted into
the IR at compile time, so each side is compiled against its own catalog).
Catalog provenance digests are deferred to M16.

## Fail-closed behavior

If a query references an operation missing from its side's catalog, the diff
returns `UIKO0004: unresolved operation reference` and exits 2 rather than
silently comparing against a wrong shape (§16.3). This is unreachable through
the CLI's compile-first flow and exists to protect programmatic callers.
