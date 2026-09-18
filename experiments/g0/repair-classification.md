# G0 repair classification v1

This vocabulary is frozen for the G0 development comparison.

A **repair iteration** begins when a validation, type-check, build, compiler or browser/acceptance observation shows that the current application does not satisfy the task, and ends after the source change(s) made to address that observation are ready for the next validation attempt.

Each repaired iteration receives exactly one primary category based on what the defect **is**, not on which technology or file it touches.

## LOCAL

A defect inside one layer or one expression/declaration/schema use.

Examples:

- wrong column label;
- wrong fallback text for a nullable phone;
- local formatting logic for order totals;
- a type error that does not connect two application layers.

## WIRING

A defect in the connection between layers or independently declared parts.

Examples:

- `customerId` route parameter not passed to `getCustomer`;
- status UI value not mapped to the OpenAPI `status` query parameter;
- page state not included in the query key/request;
- `listCustomerOrders` receives a stale customer id;
- list UI binds to the response envelope instead of `items`.

## COHERENCE

A whole-plan or contract invariant is violated even though individual declarations may look locally valid.

Examples:

- pagination uses `total` and `pageSize` inconsistently, causing Next to remain enabled past the last page;
- two reads on the same detail route resolve different logical customer identities;
- a declared component/binding contract cannot be satisfied by the imported schema;
- a change is only made valid by editing compiler/core semantics for the task.

## VISUAL_FIT

Static/semantic validation succeeds, but rendered behavior or presentation fails the task acceptance.

Examples:

- controls exist but are not reachable or visible in the browser;
- a table renders but the acceptance-required value is clipped/hidden;
- interaction works statically but the browser behavior does not match acceptance.

## ENVIRONMENT

A failure outside application semantics.

Examples:

- package registry/network outage;
- browser process crash;
- hosted model/provider failure;
- CI runner disk/resource failure.

An application build error caused by authored code is **not** ENVIRONMENT.

## Tie-break rules

Classification is conservative against uiko's claimed mechanism.

1. If LOCAL vs WIRING is genuinely ambiguous, classify **LOCAL**.
2. If WIRING vs COHERENCE is ambiguous, choose **WIRING** unless a whole-plan invariant clearly spans more than one connection.
3. Use VISUAL_FIT only when static/semantic checks are already valid and the remaining defect is browser-visible behavior/fit.
4. Use ENVIRONMENT only with evidence that the failure is external to the application.
5. If one observation reveals multiple independent defects, split them only when the transcript shows separate repair cycles. Otherwise classify the primary causal defect and record the secondary defects in the evidence text.

Two classifiers must rehearse this vocabulary on development transcripts before any final hold-out study. Raw iterations are never treated as independent experimental samples; the task remains the inferential unit.
