# uiko

## Product & Technical Architecture — v0.16

_Working specification — 18 September 2026_

> **Purpose**
>
> Define the smallest buildable architecture that can falsify or support
> uiko's product thesis: a coding agent can build and evolve UI-centric
> applications through a compact, typed specification while uiko owns
> deterministic verification and, where adopted, a managed capability plane
> for data access, authorization, audit and review-to-production identity.
>
> The architecture MUST remain general at its boundaries. Applications may use
> uiko's declarative UI, an existing React/Vue/Svelte/Angular/native UI, or
> only the managed capability plane. They may connect to arbitrary external
> systems through capability contracts and approved adapters without making
> REST, HTTP, JSON, OIDC, a frontend framework, implementation language or any
> vendor-specific protocol part of uiko core semantics.

**Status:** implementation specification for the first proof of concept plus
the normative extensibility and adoption architecture future adapters/clients
MUST preserve. This version supersedes v0.15. No measurement-window experiment
sessions have run under this version.

**v0.16 principle:** **reduce invention, compile behavior, review semantics, govern execution.**
uiko MUST NOT win merely because JSON is shorter than React, nor because a
platform centralizes auth and data access. The PoC must first show that an agent
needs to invent materially less application implementation, then that the
remaining behavior is better verified and reviewed. Only after those gates pass
may the managed runtime justify its adoption cost. The v0.14 rule remains:
**constrained core, open boundaries, explicit assurance.**

**Document conventions.** RFC 2119 keywords are normative. Security claims
state whether they apply to **Class A: agent/declarative errors**, **Class B:
hostile browser code**, or **Class C: hostile/compromised server extension
code** (§2.10). Architectural claims and competitive claims are separate:
architecture may be frozen; market claims must be re-verified before
publication (§33). Unsupported inputs fail closed.

---

# 1. Product thesis

## 1.1 The problem

Coding agents can generate frontend code. uiko does not depend on the claim
that they cannot. It targets four narrower costs that remain important for
high-iteration operational software:

1. **Agent generation burden.** In a conventional stack the agent repeatedly
   invents and maintains implementation that is structurally similar across
   features: component scaffolding, query hooks, request glue, loading/error
   state, mutation wiring, invalidation, authorization checks and transport
   policy. Even when every layer is typed, the agent still has to generate,
   reread and repair that implementation. uiko tests whether more of this
   behavior can be selected and composed from known contracts instead of
   reinvented as application code.
2. **Whole-plan verification latency.** A strong typed stack catches many local
   errors, but cross-layer wiring still spans routes, generated API types, query
   state, mutations, invalidation, authorization and component events. uiko
   tests whether those relationships can be resolved as one deterministic plan
   before browser execution.
3. **Human review bandwidth.** Agent-produced multi-file diffs are expensive to
   review repeatedly. uiko tests whether a feature-local source plus a
   complete semantic plan diff lets a reviewer reason about behavior rather than
   generated plumbing.
4. **Managed execution and trust.** Browser code should not choose arbitrary
   upstream URLs, credentials, authorization requirements, retry/idempotency
   rules or audit behavior. A runtime can centralize those concerns, but adopting
   that runtime is a real cost and MUST NOT be justified by compiler results
   alone.

The causal chain uiko is trying to prove is:

```text
less behavior invented by the agent
        ↓
smaller implementation/hypothesis space
        ↓
fewer repairs and less context to reread
        ↓
smaller, more local behavioral change
        ↓
more effective human review
        ↓
approved semantic plan
        ↓
managed execution of that exact plan
```

The staged bets are therefore:

- **Bet A1 — reduce invention:** ordinary application work requires materially
  less agent-authored implementation.
- **Bet A2 — compile behavior:** cross-layer relationships become deterministic
  compiler obligations rather than repeated browser/debug work.
- **Bet A3 — review semantics:** reviewers detect consequential behavioral
  changes more reliably from the semantic plan than from generated plumbing.
- **Bet B — govern execution:** after A1–A3 are useful, the same compiled model
  can justify a managed capability plane for authorization, audit and
  review-to-production identity.

**G0 tests A1 + A2. G1 tests A3. Significant Bet-B runtime investment is not
the default until both gates pass.**

## 1.2 Positioning

uiko is aimed at **persistent, high-iteration operational UI**: internal
admin tools, support consoles, control planes and API-backed operational
surfaces where the UI itself changes frequently.

It is not trying to beat a backend-heavy architecture when the frontend is a
thin skin. In that case the simpler default may be cheaper and better.

Adjacent patterns include:

| Pattern | Representative examples | Primary strength | Relationship to uiko |
| --- | --- | --- | --- |
| Declarative / generative UI | A2UI, json-render | constrained JSON/spec UI over approved catalogs | **not a moat**; preferred substrate/interop where it fits |
| Sandboxed embedded apps | MCP Apps | hostile-code isolation with iframe/CSP/message boundary | complementary; stronger Class B isolation than PoC custom components |
| Typed conventional frontend | React/Vue + TS + generated OpenAPI types + query library | flexible ecosystem and mature tooling | **headline engineering baseline** for G0/final measurement |
| AI-built governed app platforms | Retool, Superblocks, Windmill | fast app generation, governed data access, Git/deployment/versioning | **product competitors**; uiko must show a distinct review/plan advantage, not merely governance |
| Spec-driven application compilers | Wasp | high-level app specification tied to generated implementation | architectural precedent and authoring-format warning; Wasp retired its custom DSL for TypeScript |
| Application-model / capability infrastructure | Encore, Nitric | application model, static analysis, provider-independent resource intent | architectural precedent; validates model/adapter separation, not a product moat |
| Backend-mostly UI | server templates, htmx-style thin UI | low frontend complexity | explicitly outside the core claim |

The product claim is intentionally narrower than v0.9-era framing:

> **uiko reduces the amount of application behavior an agent must invent,
> compiles the remaining behavior into a deterministic application plan, and
> optionally governs execution of that reviewed plan.** The PoC must prove the
> reduction and review value before the managed runtime is treated as justified.

The declarative renderer is the highest-assurance and highest-automation path,
but it is not an adoption prison. Existing frontends may consume the same
managed capabilities through a stable `RuntimeClient`, and a team may adopt the
capability plane before migrating any page to uiko source (§7.10, §26.3).

**Generality is a boundary property, not a larger DSL.** "UI first" does not
mean "REST only." A REST API, GraphQL service, gRPC service, SOAP endpoint,
database, SAP/RFC system, SFTP exchange, message broker, local process,
industrial device or proprietary SDK may sit behind uiko if an approved
adapter can expose the required behavior as declared capabilities. The core
does not gain protocol-specific syntax for each one.

## 1.3 What uiko must add over a strong typed stack

The PoC does **not** claim that TypeScript stacks lack deterministic static
checks.

| Property | Strong typed frontend baseline | uiko hypothesis |
| --- | --- | --- |
| Application implementation the agent must invent | components/hooks/query/error/mutation glue are application code | ordinary plumbing supplied by compiled semantics/runtime substrate |
| Agent context that must be reread on later edits | distributed across implementation files | smaller feature-local source + stable contracts |
| Local type errors | caught by TS/schema tooling | parity expected |
| API request/response shape | generated types + validators | compiled from imported contract |
| Route/API/event/invalidation wiring | spread across layers | resolved as one plan |
| Authorization declaration coverage | usually application-specific | compiler-enforced plan invariant |
| Edit locality | often multi-file | feature-local source target |
| Review surface | textual code diff | semantic plan diff + source diff |
| Runtime transport/security policy | application code | managed runtime after G0 |

The efficiency claim therefore concerns **reduced invented implementation,
wiring, whole-plan coherence and edit shape**, not ordinary type checking and
not the trivial fact that JSON can use fewer characters than React.

A2UI/json-render already demonstrate that constrained JSON UI can reduce UI
generation surface. uiko only earns a stronger claim if the reduction also
removes cross-layer application plumbing and remains valuable after a
renderer-only control (§4.1, §24.2).

## 1.4 Trust model in one sentence

For managed paths, the browser invokes **logical IDs**. It does not choose the
upstream host, credentials, authorization requirements or mutation recovery
policy.

This structurally contains many **Class A** mistakes. It does **not** sandbox
hostile JavaScript in a same-origin custom component. Class B isolation needs
a real process/origin/iframe boundary and is post-PoC (§13.9, §17).

## 1.5 Scope of security claims

The PoC claims only what it can test:

- declarative source cannot express arbitrary browser/network code;
- built-in components do not interpret upstream strings as executable markup;
- managed API invocation resolves only compiler-approved logical operations;
- server-side authorization is independent of UI visibility;
- custom component bundles are integrity-pinned and their **declared event
  wiring** is restricted by `allowedActions`;
- hostile same-origin custom code remains trusted code and can bypass the
  bridge; it is **not isolated** by `allowedActions`, SRI or CSP;
- CSP is defense-in-depth and egress reduction, not a same-origin sandbox;
- audit/review controls can make changes observable but cannot guarantee that
  a human reviewer paid attention.

Overclaiming is a product defect.

## 1.6 Proof obligations

| # | Claim | Test | Falsified / narrowed if |
| --- | --- | --- | --- |
| 0 | uiko materially reduces implementation the agent must invent | G0 + hold-out authored-edit/final-change metrics (§4.1, §24.7) | reduction is small, comes only from renderer JSON, or shifts work to hidden/generated glue |
| 1 | Whole-plan compilation reduces cross-layer repair work | G0 + hold-out repo-twin vs B-full (§24) | paired task-level effect misses pre-registered threshold |
| 2 | Semantic plan review is materially useful | G1 pilot + completeness tests + final seeded review study (§4.2, §24.11) | semantic changes are omitted or reviewers cannot detect consequential seeded changes |
| 3 | Managed Class A paths fail closed | adversarial fixtures + compiler/runtime tests | an undeclared/unresolved managed path executes |
| 4 | Built-ins cover a useful share of target UI | hold-out expressiveness measurement | coverage below threshold with excessive custom escapes |
| 5 | Custom component implementation is framework-neutral at the contract boundary | same contract implemented in two frameworks | source/compiler semantics must change for the framework swap |
| 6 | Review maps to the deployed plan | reviewed revision + expected digest build/deploy test | runtime accepts a different plan digest |
| 7 | External-system extensibility does not require core semantic branching | synthetic non-HTTP capability adapter conformance test (§7.4–§7.11) | adding the adapter requires transport/vendor conditionals in core source semantics |
| 8 | Hybrid clients use the same managed semantics as Full | same operation invoked through renderer and plain `RuntimeClient` fixture | authz/validation/idempotency/audit outcomes diverge by client surface |
| 9 | Assurance boundaries are honest | Managed→Trusted/Unmanaged seeded diff+explain cases | downgrade is hidden, self-assertable by source, or still reported as fully managed |
| 10 | Imperfect contracts do not manufacture certainty | typed/partial/opaque compiler/runtime fixtures | opaque nested fields bind as if typed, or unsupported validation is claimed |
| 11 | Authoring format is not mistaken for the product | G0 TypeScript-spec probe against the same semantic model (§4.1) | JSONC-specific costs erase the measured advantage or require semantics unavailable through another adapter |

## 1.7 What uiko is not

- a general backend generator;
- a workflow/ETL/integration-orchestration engine;
- a visual low-code editor;
- a replacement for React, Vue, Svelte, Angular or native UI stacks;
- a hostile-code sandbox in the PoC;
- a guarantee of exactly-once effects across arbitrary upstream systems;
- a guarantee that human reviewers read carefully;
- a realtime framework in the PoC;
- a promise that every protocol/vendor receives a first-party adapter;
- a promise that arbitrary extension code is safe merely because it implements
  an uiko interface;
- a requirement that the declarative renderer be adopted before the managed
  capability plane;
- a claim that unmanaged application code inherits uiko authorization,
  validation, audit or review guarantees.

These are **assurance limits, not application limits**. A team may connect or
write whatever it needs through an approved extension or an explicitly
unmanaged escape. uiko MUST surface which guarantees apply instead of
forbidding the system or silently laundering unmanaged behavior as managed.

## 1.8 Product risks

| ID | Risk | Control / measurement |
| --- | --- | --- |
| R-GEN | uiko shortens files but does not actually reduce what the agent has to reason about/generate | instrument cumulative edits, accepted diff tokens, tool/context use and hidden glue |
| R-CORE | whole-plan compiler is not materially better than a strong typed stack | G0 before runtime build |
| R-CHASM | validation value does not convert into runtime adoption | progressive adoption profiles; do not infer runtime adoption from compiler experiment |
| R-CATALOG | built-in catalog is too small and custom escapes dominate | hold-out expressiveness + escape log |
| R-REVIEW | semantic diffs become rubber-stamped or hide semantic impact | completeness invariant + seeded review study |
| R-OPS | teams do not want a Rust runtime in their deployment path | validation-only mode remains useful alone |
| R-COMPETE | conventional tools absorb the benefit | B-full is the headline comparison, re-verified before measurement |
| R-EXT | extensibility becomes a semantic backdoor, plugin sprawl, or forces vendor-specific branches into core | capability contract + approved registry + conformance suite; source can reference extensions but cannot install executable code |
| R-ASSURE | flexibility creates hidden guarantee downgrades | compiler-derived assurance tier; `diff`/`explain` surface every downgrade; unmanaged escape requires explicit policy |
| R-ADOPT | declarative-UI requirement blocks existing applications | Full/Hybrid/Runtime-only profiles share one capability plane and invocation protocol |
| R-AUTHOR | JSONC becomes a proprietary-language tax or wins only by textual terseness | keep source adapter replaceable; run a small TypeScript-spec probe before freezing authoring identity |
| R-MOAT | benefit is already explained by json-render/A2UI or governed app platforms | renderer-only mechanism probe + explicit product comparison; do not call JSON UI or governance alone a moat |

---

# 2. Product principles

## 2.1 UI first

Backend functionality exists only where it directly supports the UI runtime:
contract-driven API invocation, validation, authorization, safe transport,
mutation recovery, audit and static asset delivery.

## 2.2 Rust owns semantics, not visual implementation

Rust is authoritative for source semantics, symbol resolution, type checking,
canonical IR, logical operations, deterministic compilation and the managed
runtime. Rust does not define how a D3 graph, editor or map is drawn.

## 2.3 Configuration is not a programming language

The authoring format is intentionally incomplete. No arbitrary JavaScript,
embedded Rust, `eval`, general loops or user-defined functions. When behavior
requires real program logic, use a real custom component behind the component
contract.

## 2.4 Compile before execute

Resolvable references, bindings, imported operation compatibility,
authorization declarations, invalidation references and component event/action
compatibility fail during validation whenever possible.

## 2.5 Stable boundaries over speculative subsystems

The long-lived boundaries are the source DTOs, canonical IR, `UiManifest`,
`RuntimePlan`, component contract, integration ports, diagnostics schema,
semantic diff and stable IDs. No marketplace, plugin ecosystem, distributed
runtime or workflow DSL in the PoC.

## 2.6 Determinism is a product feature

Identical source + imported contracts + catalog + compiler version MUST produce
byte-identical canonical plan bytes and the same digest (§9.5).

## 2.7 The catalog is governed by evidence

Built-ins are versioned. Escape usage is logged. Catalog growth is driven by
repeated evidence from real tasks, not by speculative completeness.

## 2.8 Realtime is out until it has a contract source

Realtime is post-PoC. AsyncAPI is the intended contract source when streams are
introduced.

## 2.9 Humans debug, not only review

`uiko explain` and `uiko diff` are product surfaces, not developer
conveniences.

## 2.10 Three threat classes, never conflated

- **Class A — agent/declarative errors.** The model mis-wires, hallucinates or
  produces invalid uiko source. Compiler and runtime invariants can make
  many of these states unrepresentable or non-executable.
- **Class B — hostile browser code.** Deliberately malicious code executes in
  a trusted same-origin custom component or dependency. SRI proves artifact
  identity, not benign behavior. CSP constrains selected browser capabilities,
  not the same-origin authority of the script. True isolation needs a
  sandboxed boundary, post-PoC.
- **Class C — hostile server extension code.** An in-process capability adapter
  executes with server-process authority and may bypass the managed executor,
  inspect process memory or make arbitrary network calls. It is trusted code,
  just like an in-process database driver or application plugin. The preferred
  future boundary for third-party/untrusted adapters is an out-of-process
  provider with explicit IPC, credential and network policy (§7.8).

Any security claim that does not identify the applicable class is incomplete.

## 2.11 Constrained core, open boundaries

> **Anything can be integrated. Not everything becomes uiko syntax.**

The core understands uiko-owned capability semantics, not vendor protocols.
External technologies terminate at adapters. A new adapter MUST NOT require
REST-, GraphQL-, SAP-, SQL-, hardware-, language- or vendor-specific branches
in the authoring grammar, canonical IR or renderer.

If repeated evidence shows that the existing capability kinds cannot represent
an important interaction shape, uiko may add a new **generic capability
kind**. It must not grow protocol-specific language features as a shortcut.

## 2.12 Freedom without guarantee laundering

uiko MUST allow three execution-assurance tiers (§7.9):

1. **Managed** — behavior stays inside uiko-owned compiler/runtime paths;
2. **Trusted Extension** — behavior stays inside the managed invocation
   envelope, but protocol/application execution contains approved extension
   code whose internals uiko does not prove;
3. **Unmanaged Escape** — behavior intentionally leaves the managed plane.

A lower-assurance path is not an error. A **silent** downgrade is. Any change
that reduces assurance MUST be explicit, policy-approved where required,
represented in `uiko diff`, and visible in `uiko explain`.

uiko's rule is therefore not “you cannot do that.” It is:

> **You may do it, but the system must say exactly which guarantees stop here.**

## 2.13 Progressive adoption

The managed capability plane and the declarative UI compiler share semantics
but are independently adoptable. The product supports (§7.10):

- **Full** — uiko source + renderer + managed runtime;
- **Hybrid** — existing/custom frontend + `RuntimeClient` + managed runtime,
  optionally mixed with uiko-rendered routes;
- **Runtime-only** — no uiko UI tree required; any approved client consumes
  the post-authentication `ClientContractManifest` and runtime protocol;
- **Validation-only** — compiler/CLI without the production runtime.

The same logical capability ID MUST mean the same thing across Full, Hybrid and
Runtime-only. Adoption mode must not create parallel security semantics.

## 2.14 Reduce invention before optimizing generation

The product is successful when ordinary behavior is **selected/composed from
known semantics instead of regenerated as bespoke application plumbing**. A
shorter source file is not sufficient evidence: generated hidden code, repeated
agent context reads, browser repair loops or adapter boilerplate count as work.
The experiment therefore instruments cumulative edits and accepted application
change, not only final source line count.

## 2.15 Authoring syntax is a replaceable front-end

`AppIr`, diagnostics, semantic diff and runtime semantics are the product
boundary. JSONC is the primary PoC authoring front-end because it is compact,
constrained and easy to patch deterministically. It is **not** a permanent
product identity unless evidence supports it. A TypeScript-native spec, IDE
form or future machine protocol may lower to the same located source model
without changing uiko semantics.

---

# 3. Non-negotiable invariants

1. **Rust is the semantic authority for uiko-owned semantics.**
2. **Canonical `AppIr` is private.**
3. **Public manifests are served only after authentication.**
4. **`RuntimePlan` contains logical provider identifiers, not credentials or deployment endpoints.**
5. **Managed clients invoke logical IDs, never arbitrary upstream URLs/methods/headers.**
6. **Secrets never cross the server boundary unless an explicitly approved external-provider policy grants a named secret slot.**
7. **Public IDs derive from symbolic identity, never traversal order.**
8. **Canonical plan bytes are deterministic and hashed with RFC 8785 JCS.**
9. **Ordinary declarative UI changes target one feature-local source area.**
10. **Custom UI escapes through a framework-neutral contract.**
11. **Renderer implementation is replaceable.**
12. **Unsupported contract/source constructs fail closed.**
13. **Upstream strings are data, not instructions or markup, on built-in paths.**
14. **Managed mutations require authentication, authorization, validation, durable idempotency state and audit intent before upstream execution.**
15. **`allowedActions` contains Class A event wiring only; it is not a Class B boundary.**
16. **Every followed upstream redirect in the built-in HTTP executor is re-validated against configured policy.**
17. **`RuntimePlan` is never served to clients.**
18. **Cookie-authenticated managed mutations require CSRF protection.**
19. **Every plan digest change has source/contract/catalog/extension provenance.**
20. **Every semantic plan change MUST be represented by `uiko diff`; an unexplained digest delta is a tool failure.**
21. **`uiko explain` resolves every public symbolic ID to compiled semantics and current assurance tier.**
22. **Built-ins contain no string-to-markup sink for upstream data.**
23. **The deployment build MUST match an externally stored expected plan digest associated with the reviewed source revision.**
24. **G0 occurs before implementation of the full managed trust runtime.**
25. **Experiment inference is task-clustered; nested iterations/runs do not inflate sample size.**
26. **No PoC claim treats same-origin custom code as sandboxed.**
27. **External technologies enter through capability adapters; they do not add hidden vendor/transport semantics to source, IR or renderer.**
28. **Every managed external capability has a stable logical ID, declared capability kind, input/output contract, provider identity and applicable authorization/recovery metadata.**
29. **Agent-authored source may reference only extensions already present in an operator/build-approved registry; source cannot install, download or name executable code.**
30. **Extension descriptors and normalized plan fragments are versioned, deterministic, provenance-tracked and digestable.**
31. **In-process server adapters are trusted Class C code; an interface is not a sandbox.**
32. **OpenAPI/HTTP is the first capability adapter, not an uiko semantic invariant.**
33. **`ClientContractManifest` is the only public capability contract; `UiManifest` may reference it but does not define an independent invocation model.**
34. **Full, Hybrid and Runtime-only clients use the same logical operation IDs and the same server authorization/validation/audit path.**
35. **Contract fidelity is explicit (`typed`, `partial`, `opaque`); uiko MUST NOT claim stronger static/runtime guarantees than the imported contract supports.**
36. **Assurance tier is compiler/runtime-derived from the execution path and registry, never self-asserted by agent-authored source.**
37. **A Managed/Trusted-Extension → Unmanaged downgrade is a security-significant semantic diff and requires explicit source intent plus deployment policy approval.**
38. **Unmanaged escapes never receive uiko secrets automatically and MUST NOT silently weaken the security policy of managed paths.**
39. **An opaque value may be transported, but declarative field-level binding into its unknown structure is forbidden until a contract/adapter makes that structure explicit.**
40. **Application freedom is preserved at boundaries; guarantee precision is preserved at the core.**
41. **uiko value is measured as reduced invented implementation and repair work, not merely fewer textual characters.**
42. **Authoring syntax is not canonical semantics; every supported authoring front-end MUST lower to the same located semantic model before resolution/type checking.**
43. **A renderer-only JSON control must not be confused with whole-plan compiler value.**

---

# 4. PoC gates and success criteria

## 4.1 G0 — invention + compiler-loop GO/NO-GO

G0 exists to prevent building a polished runtime around either a weak compiler
or a superficial "JSON is shorter" effect. It tests Bet A1 and Bet A2 before
substantial managed-runtime work.

**Minimum G0 slice:**

```text
JSONC source
  → Rust parser/source model
  → resolve/type/lower
  → UiManifest
  → json-render adapter + one browser host
  → imported OpenAPI read operation
  → stable diagnostics JSON
  → instrumented agent edit / validate / repair loop
```

Run 5–6 pre-registered **development tasks** against B-full. These tasks are not
the final hold-out. Capture cumulative edit payload, accepted application-owned
change, files touched, model usage when available, read/context operations,
validation repairs and browser-loop work.

G0 passes only if all are true:

- uiko shows a material reduction in **application implementation the agent
  authors**, not merely formatting/line-count compression. As a diagnostic
  target, median cumulative authored-edit tokens and accepted change tokens
  should each trend at least ~30% lower than B-full across the mini set; exact
  publication thresholds live in §24.7.
- At least one real WIRING/COHERENCE category is materially reduced and the
  work is not displaced into renderer glue, generated files or browser repair.
- Ordinary changes are observably more local and do not require compiler/core
  changes.
- The renderer mapping does not force uiko semantics to become json-render
  semantics.
- The team can explain the resulting plan and diagnostics without reading
  compiler internals.

**Renderer-only mechanism probe.** On at least three G0 tasks, run a small
control using json-render directly for UI composition while retaining
conventional TypeScript query/mutation/application plumbing. This is not a
publication arm. Its purpose is to answer: "is uiko winning only because the
UI is JSON?" If the renderer-only control captures nearly all observed
reduction, narrow the product thesis before proceeding.

**Authoring-format probe.** On at least three G0 tasks, expose the same semantic
source DTO through a minimal TypeScript object/spec facade (for the experiment
only; no second production compiler). Compare syntax/patch failures, cumulative
edit effort, diagnostics round-trip, locality and basic human readability. If
JSONC-specific friction materially erases the advantage, the authoring decision
is reopened. The goal is not to prove TypeScript or JSONC universally better; it
is to avoid confusing a replaceable syntax with uiko's semantic value.

**Default NO-GO:** if the main G0 conditions are not met, freeze the runtime
roadmap, publish the result and either narrow uiko to a smaller validation /
review tool or stop. There is no obligation to complete the rest of this
document after G0 fails.

## 4.2 G1 — semantic-review GO/NO-GO

Before significant Bet-B runtime investment, implement semantic diff
completeness for the fixture and run a small seeded review pilot using compiled
plans. Runtime execution may still be stubbed; the question is whether the plan
is a useful human review surface.

Minimum pilot:

- ≥12 diffs;
- roughly half clean and half containing one seeded consequential change;
- ≥4 reviewers where feasible, including ≥1 person not implementing uiko;
- seeds cover at least authorization widening, operation remap, invalidation
  removal and Managed→Unmanaged/contract-fidelity downgrade.

G1 GO requires:

- mechanical diff completeness: every seeded plan change is represented;
- seeded-issue detection point estimate ≥70%;
- no repeated semantic-impact class that reviewers report as unintelligible from
  the diff itself.

G1 is a product gate, not a publication-grade human study. The larger final
study remains §24.11. If G1 fails, improve or narrow the semantic-review claim
before building the full managed runtime.

## 4.3 Full PoC pass conditions

| Area | Pass condition |
| --- | --- |
| Agent generation burden | accepted/cumulative agent-authored application change is materially smaller than B-full without hidden/generated replacement work |
| Locality | ordinary field/action changes stay feature-local; no core modification |
| Authoring neutrality | TypeScript-spec probe can lower to the same semantic model; no critical compiler/runtime semantic depends on JSONC |
| Validation | invalid references fail pre-execution with stable code + source span |
| OpenAPI | one real read and one mutation operate without app-specific HTTP client code |
| Extensibility seam | one synthetic non-HTTP adapter maps query + command contracts into the same core operation model with no authoring-grammar or compiler semantic branch |
| Hybrid seam | one plain TypeScript/React client invokes a managed capability through `RuntimeClient` without consuming `UiManifest` and receives the same authz/validation behavior |
| Assurance visibility | `uiko diff` and `uiko explain` expose Managed/Trusted Extension/Unmanaged boundaries; a downgrade cannot be hidden |
| Contract fidelity | typed/partial/opaque fixtures produce the documented compile/runtime restrictions without false guarantees |
| Lists | page/offset/cursor pagination bindings compile and render |
| Renderer | uiko source/IR contain no host-framework implementation semantics |
| Custom UI | one complex component works through the Web Component ABI and explicit `allowedActions` |
| Determinism | canonical output byte-identical; RFC 8785 vectors and independent implementation checks pass |
| Review | semantic diff accounts for every plan change and G1/final seeded studies meet the pre-registered detection floor |
| Deployment identity | reviewed revision builds exactly the expected plan digest or fails |
| Authz | unscoped server invocation is rejected even if UI is manipulated |
| Mutation safety | duplicate completed requests replay; uncertain crash state never blindly re-executes an unsafe upstream mutation |
| Audit | durable audit intent/outcome records exist for every managed mutation |
| Security scope | Class A managed paths fail closed; Class B residual is documented, not mislabeled as isolated |
| Accessibility | axe-core floor + keyboard E2E |
| Expressiveness | hold-out built-in coverage meets pre-registered threshold |
| Operability | `uiko explain` resolves all public IDs in the fixture |

---

# 5. Recommended PoC baseline

| Area | Decision | Rationale |
| --- | --- | --- |
| Core | Rust 2024 | semantic authority, deterministic compiler/runtime |
| HTTP | Axum + Tokio | small, explicit Rust server stack |
| Outbound HTTP | Reqwest with explicit redirect policy | transport policy is a security boundary |
| Errors | `thiserror`; uiko-owned diagnostics | stable machine API independent of CLI renderer |
| Diagnostics UI | `miette` adapter | presentation only |
| Capability model | uiko-owned query/command contract + explicit contract fidelity | external technology is normalized before core semantic checking; unknown structure stays unknown |
| First contract adapter | OpenAPI 3.1 restricted subset | enough for realistic API-backed UIs without making OAS the core model |
| OAS parser | `oas3` + uiko strict visitor | own the supported policy, not the generic parser |
| Extension registry | explicit operator/build-approved registry | source may select approved adapters but cannot install executable code |
| Server extension form | in-process Rust for built-in/explicitly trusted PoC adapters | simplest first implementation; language-neutral external-provider boundary is post-PoC |
| Runtime validation | `jsonschema` for supported dialect/subset | server revalidation from contract-derived schemas |
| Authoring | JSONC primary PoC front-end + small G0 TypeScript-spec probe | one production path; syntax remains replaceable until evidence freezes it |
| JSONC parser/editing | Rust `jsonc-parser` with CST feature | comments/tokens/manipulation without hand-rolled CST |
| Generic renderer | `json-render` behind `JsonRenderAdapter` | first substrate; replaceable if it distorts semantics |
| Browser host | Vue 3 for first shell | host choice, not product semantics |
| Public client contract | post-auth `ClientContractManifest` + generic `RuntimeClient` | Full/Hybrid/Runtime-only share one invocation model |
| Custom component ABI | Custom Elements / Web Components | framework-neutral browser boundary |
| Digests | RFC 8785 JCS + SHA-256 digest | cross-platform canonical identity |
| Runtime storage | SQLite for PoC journal/audit | durable local state, single-instance semantics |
| Production | one Rust process/container + static assets | intentionally boring deployment |
| A11y | axe-core + keyboard E2E | minimum usability floor |
| Supply chain | `cargo audit`, `cargo deny`, pinned component digests | PoC hygiene without building a marketplace |

`jsonc-parser` is preferred over v0.11's `rowan/cstree` plan because the Rust
crate already exposes JSONC AST/CST manipulation and comment-preserving edits.
uiko should not build a syntax tree framework when the required boundary is
already available.

---

# 6. System architecture

```text
project source + imported/hand-authored capability contracts
                         ↓
            Source/Contract Adapters
        (JSONC + OpenAPI first + extensions)
                         ↓
                    Rust Compiler
       (resolve → type → policy → lower → hash)
                         ↓
                    Canonical AppIr
             /              |              \
            /               |               \
   UiManifest      ClientContractManifest    RuntimePlan
   (optional;       (post-auth public         (server-private)
   Full routes)      capability contract)          |
       |                    |                       |
       |             RuntimeClient protocol         |
       |              /          |          \      |
       |          Full        Hybrid     Runtime-only|
       |           UI          UI/client      client |
       |              \          |          /      |
       +---------------\---------|---------/-------+
                                  ↓
                             Rust runtime
                authn → authz → validate → journal/audit
                                  ↓
                        OperationExecutor router
                    /              |              \
              core adapters   trusted adapters   external providers
                    \              |              /
                     +------ provider config -----+
                     +--------- secret slots ------+
                                  ↓
                       arbitrary external systems
```

`UiManifest` is optional outside Full adoption. `ClientContractManifest` is
the stable public capability boundary for every managed client. `RuntimePlan`
is always server-private. There is one managed invocation path, not a special
“React path” or “declarative path.”

The compiler/runtime boundary remains stable if the renderer changes. The
capability boundary remains stable if an implementation changes from REST to
GraphQL, SAP, SQL, a proprietary SDK or another transport, provided its logical
capability contract remains compatible.

An explicitly Unmanaged Escape sits **outside** the managed invocation arrow.
uiko may record/review the escape declaration, but it does not pretend that
the external behavior traversed the runtime controls (§7.9).

---

# 7. Hexagonal architecture

## 7.1 Backend ports

| Port | Responsibility |
| --- | --- |
| `ProjectSource` | load source/contracts under project-root confinement |
| `CapabilityDefinitionCatalog` | resolve external contract sources through registered importers and normalize them into uiko-owned capability contracts; OpenAPI is the first importer |
| `ExtensionRegistry` | resolve approved extension IDs/versions, descriptors, provenance, config schemas and executor bindings; never installs code from project source |
| `ComponentContractCatalog` | resolve built-in/custom component contracts and expected digests |
| `OperationExecutor` | execute one compiled query/command against a resolved capability provider; concrete executors remain behind the port |
| `IntegrationConfigProvider` | logical integration/provider ID → trusted deployment configuration/transport policy; no credentials |
| `SecretProvider` | logical secret slot → credentials; never returns browser values |
| `PrincipalResolver` | authenticate caller and expose principal/scopes/session category |
| `MutationJournal` | durable idempotency state machine |
| `AuditSink` | durable mutation audit intent/outcome records |
| `AssurancePolicy` | deployment-owned policy deciding which Trusted Extension / Unmanaged escapes are permitted; never upgrades a path beyond compiler-derived evidence |

`IntegrationConfigProvider` and `SecretProvider` are deliberately separate.
Endpoint/configuration rotation is operational configuration; credential
custody is secret management. Conflating them makes audits and adapters harder.

## 7.2 Browser boundary

```text
built-ins / custom elements
          ↓
renderer application layer
          ↓
RuntimeClient (HTTP adapter | fake)
```

Built-ins never call upstream APIs directly. Custom components may contain real
application code and therefore are trusted browser code; the bridge only
constrains the declared uiko event path.

## 7.3 Composition root

One explicit composition root. No DI framework in the PoC. The composition
root owns the approved `ExtensionRegistry`; project source may reference an
extension ID already present there but may not cause dynamic code loading.

## 7.4 Capability model

The core runtime deals in **capabilities**, not transports.

PoC capability kinds:

| Kind | Meaning | Core-managed concerns |
| --- | --- | --- |
| `query` | request/response read | authn/authz, input/output validation where contract permits, deadline/error normalization |
| `command` | request/response side effect | all query controls + durable journal, recovery policy, audit and invalidation |

Post-PoC generic kinds may include `job` and `stream`. They are added only when
their lifecycle semantics are explicit; they are not approximated as unusual
HTTP requests.

Every normalized operation exposes at least:

```text
logical operation ID
capability kind
input contract + fidelity
output contract + fidelity
required authorization policy/scopes
provider/executor ID
effect/recovery metadata where applicable
audit requirement
timeout/deadline policy
provenance
compiler-derived assurance tier
```

Transport-specific details may exist in an adapter-owned **private normalized
plan fragment**, but that fragment MUST be deterministic, schema-validated,
digestable and visible to `uiko diff`/`uiko explain` as structured,
provenance-labelled data. Opaque behavior-changing bytes are forbidden.

## 7.5 Contract fidelity

External reality is not always well typed. uiko supports that reality
without fabricating certainty. Each capability input/output surface is one of:

| Fidelity | Meaning | uiko behavior |
| --- | --- | --- |
| `typed` | used structure is fully described | full compile-time binding + runtime validation for supported schema semantics |
| `partial` | known structure plus explicitly unknown remainder | checks known fields; unknown region remains opaque and cannot gain inferred guarantees |
| `opaque` | uiko cannot inspect the payload structure | transport/pass-through only; no declarative nested field binding or structural validation claim |

A fidelity downgrade is semantic and diff-visible. An adapter may improve an
opaque/partial contract by supplying a reviewed local schema, but uiko MUST
not infer fields from observed payloads and silently promote the guarantee.

Opaque values are useful for files, vendor blobs, encrypted envelopes, legacy
payloads and custom components. Generality therefore does not require
pretending every external system is JSON Schema-shaped.

## 7.6 Capability adapter contract

A capability adapter may provide one or both sides:

1. **contract importer**: OpenAPI/WSDL/protobuf/schema/custom description →
   uiko-owned capability definitions;
2. **executor**: compiled logical operation → external system invocation.

Examples include HTTP/OpenAPI, GraphQL, gRPC, SOAP, SQL, SAP/RFC, SFTP,
message systems, local processes, hardware protocols and proprietary SDKs.

The adapter owns protocol mechanics. The core still owns, where applicable:
authentication, authorization, uiko-visible validation, command
journal/audit, invocation identity, review provenance and public result
normalization.

An adapter MUST NOT smuggle a new programming language into uiko source.

## 7.7 Extension descriptor and approval

Every non-core adapter has an operator/build-approved descriptor containing at
least:

```json
{
  "extensionApiVersion": 1,
  "id": "corp.sap-rfc",
  "version": "1.0.0",
  "capabilityKinds": ["query", "command"],
  "authoringSchema": "./schemas/authoring.json",
  "deploymentConfigSchema": "./schemas/deployment.json",
  "secretSlots": ["credentials"],
  "executionBoundary": "in-process",
  "provenance": "./PROVENANCE.md"
}
```

`authoringSchema` covers plan-affecting non-secret options that may live in the
repo. `deploymentConfigSchema` covers operator-owned configuration.
`secretSlots` names credentials resolved only through `SecretProvider`.

The source may select `"adapter": "corp.sap-rfc"`. It may not select a local
binary path, container image, shell command, npm/pip/cargo URL or remote
executable. Executable supply-chain registration is outside agent-authored
source.

Assurance tier is derived from the registered execution boundary and product
policy; source cannot declare itself `Managed`.

## 7.8 Two server-extension deployment forms

**In-process trusted adapter**

- Rust implementation linked/registered with the runtime;
- simplest path for built-ins and explicitly trusted adapters;
- shares process memory, credentials and network authority;
- therefore **Class C trusted code**, not sandboxed.

**Out-of-process capability provider — post-PoC language-neutral boundary**

- implementation may be Rust, Go, Java, Python, TypeScript, C#, C++ or another language;
- communicates over a versioned local/private provider protocol;
- receives only operation input/context and named secret material explicitly
  permitted for that provider;
- may be isolated with container/OS identity, filesystem/network policy and
  resource limits;
- preserves the same logical capability contract so application source does
  not change merely because execution moves out of process.

The wire transport is not frozen in the PoC. The semantic provider envelope is:

```text
handshake(extension id/version/capabilities)
invoke(operationId, requestId, deadline, input, minimal context)
result(success | typed error | indeterminate/provider failure)
health/status
```

`job`/`stream` add lifecycle messages only when those generic kinds are
introduced.

## 7.9 Assurance tiers and escape hatches

uiko is permissive about capability, strict about **claim boundaries**.
Every execution path resolves to one tier:

| Tier | Meaning | What uiko may claim |
| --- | --- | --- |
| **Managed** | core-owned declarative/runtime path using built-in executors | documented compiler/runtime controls apply for the supported contract |
| **Trusted Extension** | managed runtime envelope invokes approved extension code | core authn/authz/journal/audit envelope applies; extension internals remain trusted Class C behavior |
| **Unmanaged Escape** | application intentionally performs behavior outside the managed runtime | only the escape declaration/provenance is reviewable; uiko makes no claim that its runtime authz/validation/audit/egress controls cover that behavior |

Rules:

- tier is derived, not author-asserted;
- `diff` MUST highlight a lower-tier transition as security-significant;
- `explain` MUST show tier and the guarantees that do/do not apply;
- an Unmanaged Escape requires explicit source intent **and** deployment policy
  approval; either side may refuse it;
- an escape receives no uiko secret slots automatically;
- reference deployments MUST NOT weaken the CSP/network policy of managed UI
  merely to accommodate an unmanaged component. Prefer a separate
  origin/process/provider boundary;
- an unmanaged path may coexist with managed paths. Its existence does not
  invalidate their guarantees if the isolation/policy boundary remains intact.

This is how uiko remains flexible without using “supports anything” as a
security euphemism.

## 7.10 Adoption profiles and `RuntimeClient`

There is one managed capability protocol and several adoption profiles:

| Profile | UI ownership | uiko production pieces |
| --- | --- | --- |
| **Full** | uiko `UiManifest` + built-ins/custom components | renderer + `RuntimeClient` + runtime |
| **Hybrid** | existing React/Vue/Svelte/Angular/native UI, optionally mixed with uiko routes | existing UI + `RuntimeClient` + runtime |
| **Runtime-only** | completely external/custom UI or client | `RuntimeClient`/protocol + runtime; no `UiManifest` required |
| **Validation-only** | existing project/toolchain | compiler/CLI only; no uiko runtime |

Full, Hybrid and Runtime-only use the same `ClientContractManifest`, logical
operation IDs and server pipeline. Hybrid is not a second, less-secure API.

A reference TypeScript `RuntimeClient` is required for the PoC hybrid proof.
Other clients may use the documented runtime protocol or later generated SDKs.
The public protocol MUST remain independent of Vue/json-render.

## 7.11 Extensibility rule

A new integration is successful when the answer to “what changed in uiko
core?” is **nothing** or a generic capability improvement useful beyond that
vendor.

Bad:

```text
if provider == SAP ...
if protocol == GraphQL ...
if device == Modbus ...
```

inside the compiler, source grammar or renderer.

Good:

```text
SAP/WSDL/protobuf/custom schema
        ↓ adapter
uiko capability contract
        ↓
unchanged compiler/runtime/client envelope
```

This is the architectural meaning of **general** in uiko.

---

# 8. Application source model

## 8.1 Feature locality and optional UI

Full application:

```text
support-console/
├── uiko.jsonc
├── features/                        # optional outside Full adoption
│   ├── customers/{module,list,detail}.jsonc
│   └── operations/dashboard.jsonc
├── integrations/                    # capability/provider declarations
│   ├── crm.jsonc
│   └── openapi/crm.json
├── components/                      # optional custom UI
│   └── network-graph/
│       ├── component.contract.json
│       ├── PROVENANCE.md
│       └── dist/
├── extensions/                      # optional reviewed adapter/provider source
│   └── corp-sap/
│       ├── extension.json
│       ├── contracts/
│       ├── provider/
│       └── PROVENANCE.md
├── extensions.lock                  # approved IDs/versions/digests; build input
├── strings/{en,es}.jsonc            # Full only
├── catalog.jsonc                    # Full only
└── theme.jsonc                      # Full only
```

A Runtime-only project may contain only `uiko.jsonc`, capability/provider
declarations, contracts, policies and extension locks. `features/`, catalog,
strings, theme and browser bundle are not mandatory. A Hybrid project may keep
its existing frontend tree beside these files.
Runtime-only MUST NOT synthesize placeholder pages merely to satisfy the compiler; absence of UI modules is a supported source shape.

An ordinary declarative customer-detail change should remain under
`features/customers/`. Capability changes should remain in their provider or
contract area.

`extensions/` is optional application-owned code/configuration, not a dynamic
plugin directory. `extensions.lock` records reviewed extension identity.
CI/deployment policy decides which implementations are approved; the runtime
does not scan and execute arbitrary contents from that directory.

## 8.2 Authoring syntax: JSONC primary, product identity not frozen

v0.12 removed an unproductive YAML-vs-JSONC bake-off. v0.16 keeps **JSONC as
the single implementation path for the PoC**, but no longer treats that choice
as evidence that uiko should permanently own a custom authoring syntax.

The source boundary is:

```text
authoring front-end
      ↓
Located Source DTOs + spans/provenance
      ↓
uiko semantic compiler
```

JSONC remains the primary front-end because it is compact, constrained and can
be patched with a lossless CST. G0 additionally runs a **minimal TypeScript-spec
probe** against the same semantic DTO (§4.1, §24.2). The probe is deliberately
small and is not a second supported production path.

The post-G0 decision MUST be explicit:

- retain JSONC as primary if its constrained surface materially helps agent
  generation/repair without imposing disproportionate human/tooling cost;
- adopt or add a TypeScript-native authoring front-end if it preserves the
  semantic advantage with materially better ecosystem/tooling ergonomics;
- in either case, `AppIr`, diagnostics and runtime semantics remain unchanged.

Accepted JSONC policy MUST be explicit. The parser must not silently accept all
loose extensions exposed by its library defaults. At minimum:

- comments: allowed;
- trailing commas: allowed;
- quoted property names: required;
- missing commas: rejected;
- single-quoted strings: rejected;
- hexadecimal numbers: rejected;
- unary-plus numbers: rejected.

## 8.3 Versioning

```jsonc
{
  "name": "support-console",
  "specVersion": 1,
  "modules": ["./features/customers"]}
```

Unknown versions fail closed. Source migration tooling is not required for the
first PoC.

## 8.4 Structured editing is part of the product

`uiko set` and `uiko patch` MUST use the JSONC CST, preserving unrelated
comments and formatting. Whole-file rewrites are allowed only when explicitly
requested or when a migration tool owns the entire rewrite.

Comment survival and locality are regression-tested.

---

# 9. Compiler architecture

## 9.1 Stages

```text
JSONC CST
  ↓
Located Source DTOs
  ↓
Resolved Model
  ↓
Typed / Coherent Model
  ↓
Canonical AppIr
  ↓
ClientContractManifest + optional UiManifest + RuntimePlan
```

`AppIr` is independent of JSONC, Vue, json-render, Axum, Reqwest and the OAS
parser implementation.

## 9.2 Pipeline

1. load project source under root confinement;
2. parse JSONC with exact spans;
3. validate `specVersion` and structural rules;
4. build symbol/module graph and reject duplicates/cycles;
5. resolve referenced capability providers through `ExtensionRegistry`;
6. load/import referenced capability contracts through
   `CapabilityDefinitionCatalog` (OpenAPI first) and component contracts;
7. normalize accepted external contracts into uiko-owned capability
   definitions;
8. resolve logical operations and schemas;
9. type-check props, bindings, route values, action inputs and event payloads;
10. validate list bindings/projection paths;
11. validate authorization declaration coverage for managed actions;
12. validate invalidation references;
13. validate custom component event→action wiring against explicit
    `allowedActions`;
14. validate extension descriptor compatibility and normalized private plan
    fragments;
15. lower to canonical `AppIr`;
16. derive deterministic `ClientContractManifest`, optional `UiManifest`, and
    private `RuntimePlan`;
17. derive contract fidelity and assurance tier from accepted contracts,
    extension registry and deployment policy inputs;
18. verify every public managed operation has one executable private-plan
    counterpart and stable logical identity;
19. attach provenance: source hashes, imported contract digests, extension
    descriptor/digest, catalog digest and compiler version;
20. canonicalize and compute plan digest;
21. emit source index and machine diagnostics.

Pure semantic passes have no filesystem/network I/O. Registry/policy data enters as explicit immutable compiler inputs.

## 9.3 Stable identity

Use symbolic IDs such as:

```text
customers.CustomerDetail.query.customer
customers.CustomerDetail.command.updateCustomer
```

Dense indexes are private implementation details only.

## 9.4 Testability by construction

Built-ins expose stable `data-uiko-*` attributes so browser acceptance tests
can address logical identities rather than CSS structure.

## 9.5 Canonical bytes and JCS

All plan identity computation MUST use RFC 8785 JSON Canonicalization Scheme.
Do not hash ordinary `serde_json` output and call it JCS.

Requirements:

- pass published RFC 8785 number/string/key-order vectors in CI;
- property-test plan-shaped values against an independent JCS implementation;
- reject NaN/Infinity and values outside the supported I-JSON discipline;
- no absolute paths, timestamps, credentials or environment-specific base URLs
  in the canonical plan;
- identical inputs + compiler version produce identical bytes and digest.

`uiko hash` prints the digest and provenance summary.

---

# 10. Public/private plan split

## 10.1 `ClientContractManifest`

Post-authentication public contract for **managed capability invocation**. It is
renderer-independent and exists in Full, Hybrid and Runtime-only modes.

May contain:

- logical query/command IDs visible to the authenticated principal;
- public input/output contract projections and fidelity (`typed`/`partial`/`opaque`);
- public normalized result/error categories;
- idempotency-key requirement indicator for commands;
- assurance tier (`Managed` / `Trusted Extension`) and provider display
  provenance safe for the client;
- protocol/version metadata required by `RuntimeClient`.

Must not contain credentials, deployment endpoints, secret refs, private
provider fragments, internal rate-limit values or authorization-policy internals.
The server may prune operations by principal.

`ClientContractManifest` is generated from the same `AppIr` as `RuntimePlan`; a
client-visible operation that has no private executable plan is a compiler
defect.

## 10.2 `UiManifest`

Optional renderer manifest used by Full/mixed routes after authentication. May
contain routes, component kinds, public props, bindings, logical operation IDs,
view-state metadata, safe theme/string references and stable IDs.

It references the same capability identities exposed by
`ClientContractManifest`; it does not invent another invocation contract.

Must not contain credentials, authorization headers, deployment endpoints,
secret refs, private rate-limit settings, raw external contracts or private
extension plan fragments.

The PoC assumes out-of-band authentication. It does not render a public login
screen from an unauthenticated uiko manifest.

## 10.3 `RuntimePlan`

Server-private. Contains compiled logical queries/actions, provider/executor
IDs, required authorization metadata, validation contracts, invalidation rules,
mutation recovery policy, assurance derivation inputs, component digests and
explicit `allowedActions`.

For the built-in OpenAPI/HTTP adapter, the provider fragment may contain
relative method/path templates and redirect-policy identifiers. Other adapters
may contribute deterministic, schema-validated **private normalized plan
fragments** (§7.4). Their semantic projection remains review-visible through
`uiko diff`/`uiko explain`.

`RuntimePlan` contains logical provider IDs, not deployment endpoints,
credentials or executable adapter locations.

## 10.4 Invocation contract

The same envelope is used by uiko renderer and Hybrid/Runtime-only clients:

```json
{
  "operationId": "customers.CustomerDetail.command.updateCustomer",
  "idempotencyKey": "01J8Z...",
  "input": { "customerId": "c-123", "name": "Alice" },
  "componentContext": "NetworkGraph"
}
```

`componentContext` is optional and non-authoritative. It exists only for
defense-in-depth on uiko component paths; external/hybrid clients omit it.
Server authorization never relies on it.

Server order for a managed command:

1. resolve logical operation ID in the compiled plan;
2. authenticate principal;
3. authorize required policy/scopes;
4. validate CSRF when session category requires it;
5. validate input to the extent contract fidelity permits;
6. inspect durable idempotency state;
7. establish durable `IN_PROGRESS` journal entry and audit intent;
8. resolve provider configuration + minimum named credentials;
9. dispatch through the compiled `OperationExecutor`;
10. validate/normalize output to the extent contract fidelity permits;
11. persist completion or indeterminate outcome;
12. write redacted audit outcome;
13. apply invalidation metadata where applicable;
14. return normalized public result.

An Unmanaged Escape does not use this envelope and therefore does not inherit
these controls (§7.9).

---

# 11. External systems and capability contracts

## 11.1 Core model: capability first, protocol second

uiko source and `RuntimeClient` bind application behavior to logical
capabilities. They do not bind UI components directly to HTTP, SQL, GraphQL,
SAP function modules, message topics, device registers or implementation
languages.

The core normalized model is intentionally small:

```text
CapabilityProvider
  ├── query operations
  └── command operations
       (job / stream reserved for explicit later lifecycle semantics)
```

Each operation has stable identity, kind, contract fidelity, policy metadata,
provider identity, effect/recovery metadata and provenance (§7.4–§7.5). The
external transport is an adapter concern.

This lets the same application contract survive implementation changes such as:

```text
REST service        → GraphQL service
vendor SDK          → internal gRPC gateway
in-process adapter  → out-of-process Java/Python provider
PostgreSQL          → proprietary mainframe service
```

when the logical contract remains compatible. A migration that lowers contract
fidelity or assurance tier is not “compatible by silence”; it appears in the
semantic diff.

## 11.2 OpenAPI adapter — first implementation, fail closed

The PoC's first contract adapter supports a deliberate OpenAPI 3.1 subset:
primitive JSON types, arrays/objects, required/optional fields, path/query
parameters, JSON request bodies, selected JSON success responses and
declarative list bindings.

Unsupported input fails with a stable diagnostic. No silent approximation.

Example:

```jsonc
{
  "integrations": {
    "crm": {
      "adapter": "uiko.openapi-http",
      "contract": "./openapi/crm.json",
      "operations": {
        "listCustomers": {
          "listBinding": {
            "pagination": {
              "strategy": "cursor",
              "cursorParam": "cursor",
              "sizeParam": "limit",
              "defaultSize": 25,
              "maxSize": 100
            },
            "projection": {
              "itemsPath": "data.items",
              "cursorPath": "data.nextCursor"
            }
          }
        }
      }
    }
  }
}
```

The OpenAPI adapter validates configured parameters and projection paths and
normalizes accepted operations into the same uiko capability model used by
every future adapter.

## 11.3 OpenAPI 3.1 and JSON Schema wording

OpenAPI 3.1 Schema Objects are based on JSON Schema Draft 2020-12 through the
OAS dialect; OpenAPI also defines OAS-specific vocabulary, and documents or
schema resource roots may select dialects with `jsonSchemaDialect` / `$schema`.

Therefore the PoC MUST NOT state that "OpenAPI 3.1 equals Draft 2020-12".
Instead:

- `oas3` parses the OpenAPI document;
- an uiko strict visitor recognizes the supported OAS 3.1 dialect/subset;
- unsupported alternate dialects or unsupported keywords fail closed;
- uiko lowers supported schemas into its own capability/type model;
- runtime validation uses the JSON Schema implementation only for the subset
  uiko has explicitly accepted.

## 11.4 Other systems

The following are **architectural extension targets, not v0.16 implementation
claims**:

| External system | Natural adapter path |
| --- | --- |
| REST/HTTP | OpenAPI contract importer + HTTP executor — PoC implementation |
| GraphQL | schema/operation importer + GraphQL executor |
| gRPC | protobuf/service importer + gRPC executor |
| SOAP | WSDL importer + SOAP executor |
| SQL/database | reviewed query/command contract + database executor; never browser-supplied arbitrary SQL |
| SAP/RFC / ERP SDK | generated or hand-authored capability contract + vendor adapter |
| SFTP/files | file capability adapter with bounded paths/operations |
| Kafka/MQTT/message systems | stream/command adapter once stream semantics exist |
| local process/desktop bridge | approved external capability provider |
| PLC/OPC-UA/Modbus/hardware | device-specific provider exposing typed logical capabilities |
| proprietary internal SDK/protocol | organization-owned adapter/provider |

uiko does not need to understand those protocols. It needs their adapter to
declare behavior in a form the compiler/runtime can reason about.

If an organization cannot or does not want to place a behavior behind a
capability adapter, it may use an **Unmanaged Escape**. That is supported as an
application composition choice, not represented as a fake capability. The
boundary must remain explicit (§7.9).

## 11.5 Streams and long-running work

Any `stream` or `job` capability is a compile error in the PoC.

Post-PoC:

- `stream` introduces explicit subscription/reconnect/buffer lifecycle;
- AsyncAPI is the preferred first contract source where it fits;
- `job` introduces start/status/cancel/result semantics rather than pretending
  a twenty-minute operation is a normal request/response command.

These are generic capability kinds, not transport-specific exceptions.


---

# 12. Renderer architecture

## 12.1 Responsibilities

The renderer consumes `UiManifest`, creates built-ins, manages query/action
view state, performs public pre-validation, dispatches logical runtime calls,
mounts custom elements, applies theme/string catalogs and preserves
accessibility behavior.

It MUST NOT parse uiko source/external contracts or choose upstream endpoints.

## 12.2 `json-render` is the first substrate, not a gate

v0.12 changes the default from "evaluate json-render" to:

```text
UiManifest → JsonRenderAdapter → json-render → Vue host
```

`json-render` already provides framework renderers and catalog/action concepts.
uiko should reuse that machinery when it can do so without changing its own
semantics.

The adapter is retained as an anti-coupling boundary. Replace `json-render` if
any of these become true:

- uiko must rename/redefine core semantics to fit it;
- unsupported uiko behavior accumulates custom patches inside the adapter;
- debugging requires understanding json-render internals instead of uiko
  stable IDs;
- the adapter owns business/security semantics that belong in the compiler.

## 12.3 Vue is only the first host

No Vue SFC names, refs, composables, lifecycle APIs or CSS framework classes in
uiko source/IR.

## 12.4 Built-in list contract

Table/list built-ins consume the compiled list binding. Browser code does not
invent pagination parameter names or response projection rules.

## 12.5 Markup discipline

- built-ins do not use `innerHTML`, `v-html` or equivalent sinks for upstream
  strings;
- upstream text renders as text;
- if Trusted Types is enabled, the policy MUST NOT become a generic escape
  hatch that accepts arbitrary upstream HTML;
- provenance marking is available for externally sourced display values.

## 12.6 Minimal i18n

String catalogs are referenced from the manifest. Locale selection is runtime
state. Full ICU pluralization/RTL infrastructure is post-PoC.

## 12.7 Version-skew recovery

Manifest/runtime protocol mismatch → refetch once → retry once → explicit
error. Never silently partially render incompatible plans.

---

# 13. Framework-neutral custom components

Custom components are the escape valve for UI that does not belong in the
built-in declarative catalog.

## 13.1 Contract

```json
{
  "specVersion": 1,
  "name": "NetworkGraph",
  "element": "uiko-network-graph",
  "allowedActions": ["operations.Dashboard.action.selectNode"],
  "props": {
    "nodes": { "schema": { "type": "array" }, "required": true },
    "edges": { "schema": { "type": "array" }, "required": true },
    "selectedNode": { "schema": { "type": ["string", "null"] } }
  },
  "events": {
    "node-selected": {
      "detail": {
        "type": "object",
        "required": ["nodeId"],
        "properties": { "nodeId": { "type": "string" } }
      }
    }
  },
  "slots": []
}
```

No action groups in the PoC. If repeated real components produce large,
cohesive allowlists, grouped approvals may be designed later from evidence.

## 13.2 ABI rules

The PoC browser ABI is explicit:

- **complex props** (objects/arrays) are assigned as DOM properties, never
  JSON-stringified attributes;
- primitive reflective configuration MAY use attributes when the contract says
  so, but attribute representation is not the semantic source of truth;
- outbound events use `CustomEvent<T>` and place typed payload in `.detail`;
- uiko events MUST use `bubbles: true` and `composed: true` so the host bridge
  can receive them across Shadow DOM;
- slots use native slot semantics only;
- lifecycle is ordinary Custom Element `connectedCallback` /
  `disconnectedCallback`; no uiko-specific framework lifecycle;
- Shadow DOM is optional rendering encapsulation, never a security boundary.

## 13.3 Event/action enforcement

Compiler: source may wire declared component events only to actions listed in
that component contract's `allowedActions`.

Bridge: normal event dispatch outside the allowlist is rejected and logged.

Runtime: a provided `componentContext` may be checked again as defense-in-depth.
It is not authoritative against same-origin hostile code.

## 13.4 Framework adapters

React, Vue, Svelte, Angular or vanilla implementations compile/wrap to the same
Custom Element contract. Convenience templates live outside compiler core.

## 13.5 Review rule for the PoC

The PoC optimizes for correctness, not rebuild automation:

- any contract change requires human review;
- any custom component bundle digest change requires human review;
- `PROVENANCE.md` records source revision, build command and reviewer;
- SRI/digest mismatch refuses mount.

The v0.11 "same contract + attestation = no human review" optimization is
post-PoC. There is no reason to build an attestation ecosystem before there is
rebuild volume.

## 13.6 Example usage

```jsonc
{
  "type": "NetworkGraph",
  "props": {
    "nodes": { "from": "network", "path": "nodes" },
    "edges": { "from": "network", "path": "edges" }
  },
  "on": { "node-selected": { "action": "selectNode" } }
}
```

Changing the implementation from React to Svelte must not require uiko
source or compiler semantic changes when the contract is unchanged.

## 13.7 Same-origin reality

A same-origin custom element shares the page's JavaScript authority. It can
call same-origin endpoints directly, inspect accessible page state and bypass
the event bridge. `allowedActions` protects the normal uiko wiring path;
it does not create capabilities that hostile code cannot bypass.

## 13.8 SRI reality

SRI/digest pinning establishes **which bundle** was loaded. It does not
establish that the bundle is benign. Source review and supply-chain controls
remain required.

## 13.9 Future hostile-code mode

A post-PoC Sandboxed Embed mode may place untrusted custom UI in a sandboxed
iframe/origin with a narrow message/capability bridge. The existing Component
Contract should be reusable as the interface, but the isolation mechanism is a
separate deployment architecture.

---
# 14. Realtime — post-PoC

Any stream declaration is rejected by the PoC compiler.

Future direction:

```text
AsyncAPI
  ↓
Realtime contract adapter
  ↓
logical stream IDs
  ↓
managed runtime transport
```

The runtime, not browser source, should own target endpoint, authentication,
reconnect, heartbeat, validation, throttling and buffering policy.

---

# 15. Runtime result and state model

## 15.1 Public result model

```text
RuntimeResult<T>:
  Success
  ValidationError
  AuthorizationError
  CsrfError
  UpstreamError
  Timeout
  RateLimited
  DuplicateCompleted
  MutationInProgress
  IndeterminateMutation
  ActionNotAllowedForComponent
  InternalError
```

Transport/library internals are never exposed directly to the browser.

## 15.2 Renderer state

```ts
type QueryState<T> =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "success"; data: T }
  | { status: "error"; error: RuntimeError };
```

Actions use equivalent submitting/success/error states. Do not introduce a
global state framework until a measured coordination problem requires one.

## 15.3 Declarative invalidation

Actions declare `invalidates: [...]`; the compiler validates referenced query
IDs. The runtime returns invalidation metadata after a committed success. The
renderer decides how to refetch/invalidate local cached views.

## 15.4 Client-side pre-validation

Public input schemas may be included in `UiManifest` for fast form feedback.
The server always validates again.

## 15.5 Durable idempotency: no exactly-once fiction

A local idempotency key does not make an arbitrary upstream API exactly-once.
The failure window is fundamental:

```text
runtime sends mutation
  ↓
upstream commits
  ↓
process/network fails before runtime records success
```

Blindly retrying can duplicate the mutation.

The PoC therefore uses a durable SQLite-backed mutation journal keyed by
`(principal, actionId, idempotencyKey)` with at least:

```text
IN_PROGRESS
COMPLETED(result_digest + replayable public result)
INDETERMINATE(reason + correlation data)
```

Rules:

1. create `IN_PROGRESS` durably before the upstream request;
2. if `COMPLETED`, replay the recorded public outcome;
3. if an identical request is already actively `IN_PROGRESS`, return
   `MutationInProgress` rather than start another call;
4. on clean upstream failure known not to have committed, persist the safe
   failure outcome as appropriate;
5. after crash/restart, a stale `IN_PROGRESS` is **not automatically retried**;
6. if the upstream operation supports an idempotency key/header and the
   integration contract maps uiko's key to it, recovery MAY safely retry
   according to that declared policy;
7. otherwise stale `IN_PROGRESS` becomes `INDETERMINATE` and requires an
   application/operator reconciliation path.

This is at-least/at-most-once aware orchestration, not a claim of distributed
exactly-once semantics.

## 15.6 Rate limiting scope

The PoC runtime is **single-instance**. Per-principal and global rate limits are
valid only for that instance. Multi-replica/distributed limits require shared
state and are post-PoC.

Rate limits are still valuable for bounded abuse and accidental loops, but the
document must not present local counters as a distributed control.

## 15.7 Audit durability

Before a managed mutation reaches the upstream executor, the runtime MUST
persist both:

- mutation journal `IN_PROGRESS`;
- audit intent containing principal/action/correlation ID and redacted input
  summary.

If either durable write fails, the mutation fails closed before upstream
execution.

Completion/indeterminate outcome is appended/persisted afterwards. Audit is not
best-effort `tracing` output for managed mutations.

## 15.8 Version skew

Protocol/manifest mismatch → refetch once → retry once → explicit error.

---

# 16. Diagnostics, diff and explainability

## 16.1 `uiko validate`

Diagnostics are a public machine interface:

```text
UIKO2104 features/customers/detail.jsonc:18:16
unknown operation `crm.getCustmer`

Did you mean:
  crm.getCustomer

Expected input:
  customerId: string

Current binding:
  route.id: string
```

`uiko validate --format json` MUST emit versioned structured diagnostics
with stable code, source range, message, expected/actual context, related spans
and optional high-confidence suggestions.

Agents never need to scrape ANSI terminal output.

## 16.2 `uiko explain`

```text
$ uiko explain customers.CustomerDetail.command.updateCustomer

operation:    customers.CustomerDetail.command.updateCustomer
kind:         command
provider:     integrations.crm / uiko.openapi-http
assurance:    Managed
input:        typed
output:       typed
transport:    POST /customers/{id}              # provider projection
resolved-at-runtime: <redacted/configured endpoint class>
input bindings:
  customerId  string required ← route.customerId
  name        string required ← form.name
authorization: crm:write
invalidates:  customers.CustomerList.query.listCustomers
mutation:
  client idempotency key required
  upstream key propagation: yes/no
source:       features/customers/detail.jsonc:42
```

For Trusted Extension and Unmanaged paths, `explain` MUST name the boundary and
state the controls uiko does **not** claim beyond it. For partial/opaque
contracts it MUST identify which portions are not statically/runtime validated.

`explain` reads the compiled plan and approved registry/policy inputs. Runtime-
sensitive values are redacted unless an explicitly privileged local admin mode
is used.

## 16.3 `uiko diff` is a completeness contract

The semantic diff is not just pretty output. It MUST represent every semantic
change that contributes to the plan digest.

At minimum it reports changes to:

- public capability additions/removals and `ClientContractManifest` changes;
- routes/pages/components when `UiManifest` exists;
- operation bindings;
- input/output projections and contract-fidelity changes;
- authorization requirements;
- assurance-tier changes and Unmanaged Escape declarations;
- invalidation;
- mutation/idempotency policy;
- component contracts and `allowedActions`;
- imported contract/catalog/extension digest provenance.

A Managed → Trusted Extension or any → Unmanaged transition MUST be rendered as
a security-significant row with the controls lost/changed, not merely as an
extension/provider ID change.

Invariant:

> If `oldPlanDigest != newPlanDigest` and `uiko diff` cannot attribute the
> difference to an explicit semantic/provenance row, `uiko diff` fails
> closed with `UIKO_DIFF_INCOMPLETE`.

A bare digest delta is never sufficient review material.

`uiko diff --format json` is required so CI/review tooling can reason about
semantic impact without scraping text.

---

# 17. Security model

## 17.1 Scope

The security architecture covers the **managed data plane** and, where used,
built-in renderer paths. Full, Hybrid and Runtime-only managed clients enter
the same server pipeline. Custom same-origin browser code is trusted Class B
application code, not sandboxed code. In-process server capability adapters are
trusted Class C application/infrastructure code, not sandboxed code.

An **Unmanaged Escape is deliberately outside this security boundary**. uiko
may review/record that the boundary exists; it does not claim that managed
authz, validation, journal, audit or egress controls executed on that path.

UI visibility is never authorization. Extension registration is never
authorization either: runtime authorization still applies before a managed
operation is dispatched.

## 17.2 Authentication and authorization

`PrincipalResolver` supplies authenticated principal, scopes and session
category. Server rule:

```text
operation.requiredScopes ⊆ principal.scopes
```

This check occurs before upstream execution regardless of what the manifest or
DOM displays.

## 17.3 CSRF

- cookie/session authentication + mutation → CSRF token/strategy required;
- bearer/header token flows are not protected by browser cookie CSRF semantics
  and declare their category explicitly;
- deployment adapters MUST NOT silently guess the authentication category.

## 17.4 Threat model

| ID | Class | Threat | PoC control | Residual |
| --- | --- | --- | --- | --- |
| T-A-WIRE | A | agent binds nonexistent/wrong logical operation | compiler resolution/type/coherence | rejected before execution |
| T-A-COMP | A | component event wired outside declared `allowedActions` | compiler + bridge + optional runtime hint check | normal uiko path rejected |
| T-A-PAYLOAD | A | poisoned upstream string interpreted as machine instruction/markup | declarative source + text rendering + no string-to-markup sinks | human social engineering remains |
| T-A-REDIRECT | A/B | upstream redirect escapes configured integration hosts | custom Reqwest redirect policy validates every hop | implementation bug remains test target |
| T-B-CUSTOM | B | hostile same-origin custom bundle bypasses bridge and calls same-origin runtime | source review, digest pinning, server authz, rate limits, audit | **not isolated**; sandboxed mode post-PoC |
| T-B-SUB | B | unexpected component bundle substituted | digest/SRI check | approved malicious bundle still malicious |
| T-C-ADAPTER | C | hostile/compromised in-process server adapter bypasses managed executor assumptions, reads process state or performs arbitrary egress | operator-approved registry, provenance/digest pinning, code review; prefer external-provider boundary for third-party adapters | **not isolated in-process**; trusted runtime code |
| T-C-PROVIDER | C | compromised out-of-process capability provider abuses the authority delegated to it | explicit invocation protocol, minimal credential/config delegation, deployment network/process policy, audit at core boundary | provider can still misuse legitimately delegated capability |
| T-U-ESCAPE | out-of-boundary | managed-looking application behavior silently bypasses uiko through direct/custom I/O | explicit escape declaration, dual source+deployment approval, diff/explain warning, no implicit secret slots, isolation policy | **uiko controls do not cover the escaped behavior** |
| T-CSRF | B | cross-site mutation on cookie session | CSRF control | adapter/config errors remain |
| T-PLAN | B | private plan exposed to browser | no serving route; asset scan regression | server compromise outside PoC claim |
| T-REVIEW | — | reviewer misses semantically dangerous small diff | complete semantic diff + seeded study | human attention not guaranteed |
| T-IDEMP | — | mutation commits upstream but runtime loses outcome | durable journal + upstream idempotency mapping or indeterminate state | operator reconciliation may be required |
| T-AUDIT | — | mutation happens without durable audit evidence | persist audit intent before execute; fail closed | storage compromise outside PoC scope |

## 17.5 Network egress

Managed built-ins communicate same-origin to the uiko runtime. The core
runtime dispatches only compiled logical capabilities.

For the built-in OpenAPI/HTTP executor:

- base endpoint originates from `IntegrationConfigProvider`, not browser input;
- credentials originate from `SecretProvider`;
- path/query/body are compiled operation inputs;
- every redirect hop is policy-validated;
- request/response sizes and timeouts are bounded.

For extensions:

- project source may only select an adapter already approved in
  `ExtensionRegistry`;
- deployment configuration and secrets remain operator-owned;
- remote contract/code fetching is disabled in the PoC;
- local project/contract paths are project-root confined;
- an **in-process custom adapter is Class C trusted code** and can technically
  bypass the HTTP executor's network policy by using its own networking APIs;
- an out-of-process provider can be constrained with a narrower deployment
  network/credential policy, but this is post-PoC and not automatic safety.

Likewise, this does not imply a hostile same-origin custom component is unable
to use other browser mechanisms. That is Class B and post-PoC sandbox
territory.

For an approved Unmanaged Escape, network/process authority is deployment-owned
and MUST be isolated so enabling the escape does not silently broaden managed
paths. The reference browser deployment does not relax its global CSP merely
for one unmanaged component; use a separate origin/process/provider or an
explicitly reviewed deployment exception.

## 17.6 Component integrity

Registered bundle digest is part of build/plan provenance. Mount fails on
mismatch. The PoC does not auto-fetch remote component code.

## 17.7 CSP baseline

CSP is defense-in-depth, not the Class B isolation mechanism.

Minimum production policy is deployment-specific but SHOULD start from an
explicit baseline such as:

```text
default-src 'self';
script-src 'self';
connect-src 'self';
img-src 'self' data:;
style-src 'self';
font-src 'self';
object-src 'none';
base-uri 'none';
form-action 'self';
frame-src 'none';
worker-src 'self';
frame-ancestors 'none';
require-trusted-types-for 'script';
trusted-types uiko-policy;
```

Important: `form-action` has no `default-src` fallback and therefore must be
set explicitly if form navigation is to be constrained. Similar navigation /
embedding directives are treated explicitly rather than assuming
`default-src` covers all browser egress.

If inline styles/scripts are required by a chosen renderer/toolchain, loosen
the policy deliberately with nonces/hashes or documented exceptions; do not
silently add `'unsafe-inline'` as a framework convenience.

## 17.8 Human-channel threat

External text may still tell a human operator to perform a dangerous action.
uiko can mark provenance and avoid interpreting that text as code, but it
cannot solve social engineering by rendering rules alone.

## 17.9 Plan confidentiality

`RuntimePlan` and private extension fragments are server-private and absent from
browser/static asset outputs. `ClientContractManifest` is intentionally public
post-auth; its contents are constrained by §10.1. CI scans static artifacts for
forbidden private plan material.

## 17.10 STRIDE-lite deliverable

Maintain a compact threat table mapped to executable regression/adversarial
cases. If a threat lacks either an explicit residual or a testable control,
its wording is incomplete.

---

# 18. Build and deployment model

## 18.1 Development loop

```text
edit source
  ↓
uiko validate --format json
  ↓
repair
  ↓
uiko dev
  ↓
acceptance loop when needed
```

The browser loop complements static validation; it does not replace it.

## 18.2 Production artifact

`uiko build` emits according to the selected/adopted surface:

- Rust runtime binary when using Full/Hybrid/Runtime-only managed runtime;
- optional static renderer bundle for Full routes;
- embedded `RuntimePlan`;
- post-auth `ClientContractManifest` artifact/seed;
- optional `UiManifest` artifact/seed when declarative routes exist;
- build manifest with compiler/source/contract/catalog/extension digests;
- plan digest.

A Runtime-only build therefore has no mandatory browser bundle. No
Node/Deno/Vite process is required at production runtime.

## 18.3 Embedded plan only in the PoC

v0.11 supported embedded and signed-external plans. v0.12 removes the external
mode from PoC scope.

Reason: signed hot-swappable plans introduce key custody, signature workflows,
rollback/version semantics and a second deployment path before product value is
proven.

The PoC has one plan path: **embedded immutable plan per artifact**.

External/signed plans may return post-PoC if users demonstrate a real need for
rule/graph updates without rebuilding the runtime artifact.

## 18.4 Review-to-production identity

A hash is useful only if the reviewed digest cannot be silently replaced by the
same actor/path that generated the change.

PoC deployment procedure:

1. human reviews source diff + `uiko diff` for a specific source revision;
2. review system records `(sourceRevision, expectedPlanDigest)` in protected CI
   environment/release metadata outside the agent-writable repository tree;
3. CI checks out exactly `sourceRevision`;
4. `uiko build --expect-plan <digest>` recompiles deterministically;
5. build fails if produced plan digest differs;
6. artifact metadata embeds both source revision and plan digest;
7. runtime startup verifies embedded plan bytes match embedded build metadata.

The approval event is therefore source-revision + semantic-digest specific.
The PoC does not pretend that a digest committed by the agent next to its own
source constitutes independent approval.

## 18.5 Build manifest

Records at least:

```text
compiler version
toolchain lock digest
source revision + source digest
imported capability-contract digests
extension registry/lock digest
extension descriptor/provider digests as applicable
catalog digest
custom component contract/bundle digests as applicable
ClientContractManifest digest
UiManifest digest when present
RuntimePlan digest
```

No secret material.

## 18.6 Runtime configuration

Deployment-owned provider configuration resolves through
`IntegrationConfigProvider`. Credentials resolve through `SecretProvider`.

For HTTP this includes base URL/host policy. For another adapter it may include
a database DSN identifier, broker address, device endpoint, SAP system name or
other non-secret deployment setting defined by the adapter's
`deploymentConfigSchema`.

Changing deployment configuration or rotating credentials does not change plan
semantics or require a rebuild, provided the logical provider identity and the
compiled adapter policy remain satisfied. Changing adapter ID/version,
plan-affecting authoring options or imported capability contracts **does**
change plan provenance/digest.
## 18.7 Deployment topology

PoC runtime: one Rust container/process + one SQLite volume. Full mode may
serve static assets from that process; Hybrid/Runtime-only may serve no UI at
all. Approved external capability providers, if demonstrated later, are
separate processes/containers.

This is intentionally single-instance. Kubernetes multi-replica semantics,
shared journals, distributed rate limits and HA storage are post-PoC.

---

# 19. Agent workflow

## 19.1 Ordinary feature change

```text
agent reads feature source + relevant contract summary
          ↓
uiko patch / edit JSONC
          ↓
uiko validate --format json
          ↓
repair stable diagnostics
          ↓
uiko diff
          ↓
acceptance browser loop if visual/behavioral proof is needed
```

The desired ordinary change does not require writing HTTP client plumbing,
duplicate API DTOs, retry booleans, authorization code or cache invalidation
wiring.

## 19.2 Complex component

```text
identify built-in gap
  ↓
write real component in suitable framework
  ↓
wrap/compile as Custom Element
  ↓
define component.contract.json
  ↓
review source + bundle
  ↓
pin digest + provenance
  ↓
use declaratively from uiko source
```

## 19.3 Framework choice belongs to the component

| Need | Example implementation |
| --- | --- |
| ordinary form/table/dashboard | built-in catalog |
| existing corporate Angular widget | Angular → Custom Element wrapper |
| D3 graph | vanilla/React/Vue/Svelte implementation |
| highly reactive compact widget | Svelte Custom Element |
| existing Vue design-system widget | Vue Custom Element |
| WebGL scene | Three.js/custom wrapper |

Framework duplication/weight remains a real deployment trade-off; framework
neutrality is a compiler-contract property, not a promise of zero bundle cost.

## 19.4 Hybrid / Runtime-only adoption workflow

An existing application does not need to rewrite its pages:

```text
existing React/Vue/Angular/native UI
            ↓
      RuntimeClient
            ↓
ClientContractManifest logical operation
            ↓
      uiko runtime
            ↓
   capability provider
```

Typical migration is capability-by-capability or route-by-route:

1. import/define one external capability contract;
2. compile and review `ClientContractManifest` + `RuntimePlan`;
3. replace one direct API call with `RuntimeClient`;
4. gain managed authz/validation/audit for that call;
5. optionally migrate its UI to uiko source later;
6. leave unrelated frontend/backend behavior untouched.

The product must never require a flag day.

## 19.5 Local models

A secondary experiment may test whether deterministic diagnostics make smaller
local models sufficient for ordinary edits. This is not required to pass G0
and is not a headline product claim unless measured.

---

# 20. Extension packaging and provenance

## 20.1 Custom UI component package

```text
network-graph/
├── component.contract.json
├── PROVENANCE.md
├── package.json            # only if implementation build needs it
└── dist/
    ├── component.js
    └── component.css       # optional
```

Project registration specifies contract path, bundle path and expected digest.
No runtime directory scanning, remote package marketplace or remote code
fetching in the PoC.

`PROVENANCE.md` records source repository/revision, build command/toolchain,
reviewer, bundle digest and contract digest.

## 20.2 Server capability extension package

An application or organization may own an adapter:

```text
corp-sap/
├── extension.json
├── schemas/
│   ├── authoring.json
│   └── deployment.json
├── contracts/
├── provider/               # Rust adapter or external provider source
├── tests/
└── PROVENANCE.md
```

The package may live in the application repo, a separate internal repo, or a
published registry artifact. **Location is not approval.** The build/deployment
`ExtensionRegistry` decides the exact trusted ID/version/digest.

A project can therefore keep unusual integrations under normal Git/PR/CODEOWNERS
control without forking uiko core.

## 20.3 No executable indirection from source

uiko source may refer to:

```jsonc
{
  "adapter": "corp.sap-rfc",
  "contract": "./contracts/customer-service.json"
}
```

It MUST NOT refer to:

```text
shell command
arbitrary binary path
container image chosen by source
npm/pip/cargo package URL
remote script/plugin URL
```

Those are executable supply-chain choices and belong to reviewed
build/deployment configuration.

## 20.4 Extension conformance

Every capability extension intended for managed runtime use must pass a common
conformance suite covering, as applicable:

- descriptor/version compatibility;
- deterministic contract normalization;
- stable logical operation IDs;
- input/output contract fidelity behavior (`typed`/`partial`/`opaque`);
- query vs command effect classification;
- required authorization metadata;
- command recovery/idempotency declarations;
- structured diagnostics;
- deterministic, review-visible private plan fragment;
- `uiko explain` projection;
- runtime error normalization;
- provenance/digest reporting;
- assurance-tier derivation and downgrade visibility.

Passing conformance does not make hostile extension code safe. It proves
interoperability and declared behavior, not benevolence.


---

# 21. TDD strategy

## 21.1 Lowest valuable boundary

| Layer | Test style |
| --- | --- |
| JSONC adapter | policy, spans, comment-preserving patch, root confinement |
| Compiler | resolution, type/coherence checks, deterministic lowering |
| OpenAPI adapter | supported dialect/subset contract tests |
| Component contracts | prop/event schema + `allowedActions` checks |
| Runtime use cases | in-memory ports/fakes where durability is not the subject |
| Mutation journal | real temporary SQLite, crash/recovery state-machine tests |
| Audit sink | real temporary SQLite/file sink, fail-closed tests |
| HTTP executor | controlled local upstream server for timeout/redirect/idempotency behavior |
| Renderer app | binding/state/logical dispatch tests |
| Built-ins | DOM + accessibility behavior |
| Custom bridge | property assignment + `CustomEvent.detail` + allowlist path |
| Whole PoC | small deterministic Playwright acceptance set |

## 21.2 Core regression set

At minimum:

```rust
#[test] fn jsonc_policy_rejects_loose_unapproved_extensions() {}
#[test] fn uiko_patch_preserves_unrelated_comments_and_formatting() {}
#[test] fn unknown_operation_fails_with_stable_span_diagnostic() {}
#[test] fn wrong_list_projection_fails_before_browser_execution() {}
#[test] fn action_without_required_scopes_is_rejected_server_side() {}
#[test] fn cookie_mutation_without_csrf_is_rejected() {}
#[test] fn redirect_to_disallowed_host_is_not_followed() {}
#[test] fn custom_event_outside_allowedactions_fails_compile() {}
#[test] fn custom_bundle_digest_mismatch_refuses_mount() {}
#[test] fn hostile_custom_code_is_not_claimed_as_bridge_contained() {}
#[test] fn completed_idempotency_key_replays_recorded_outcome() {}
#[test] fn concurrent_duplicate_returns_mutation_in_progress() {}
#[test] fn stale_in_progress_without_upstream_idempotency_becomes_indeterminate() {}
#[test] fn stale_in_progress_with_declared_upstream_key_may_reconcile_safely() {}
#[test] fn audit_storage_failure_prevents_upstream_mutation() {}
#[test] fn plan_digest_passes_rfc8785_vectors() {}
#[test] fn plan_digest_matches_independent_jcs_on_random_plan_corpus() {}
#[test] fn identical_inputs_build_byte_identical_plan() {}
#[test] fn semantic_plan_change_without_diff_row_fails_diff_completeness() {}
#[test] fn build_fails_when_expected_reviewed_plan_digest_differs() {}
#[test] fn public_manifests_are_not_served_before_authentication() {}
#[test] fn runtime_plan_is_absent_from_static_assets() {}
#[test] fn complex_props_are_assigned_as_dom_properties() {}
#[test] fn component_events_use_typed_custom_event_detail() {}
#[test] fn framework_swap_preserving_contract_needs_no_source_change() {}
#[test] fn builtins_have_no_upstream_string_to_markup_sink() {}
#[test] fn unregistered_extension_id_fails_closed() {}
#[test] fn source_cannot_select_executable_extension_path_or_remote_code_url() {}
#[test] fn synthetic_non_http_adapter_normalizes_without_core_protocol_branch() {}
#[test] fn extension_plan_fragment_is_deterministic_and_diff_visible() {}
#[test] fn client_contract_manifest_and_runtime_plan_share_operation_identity() {}
#[test] fn hybrid_runtimeclient_hits_same_authz_path_as_full_renderer() {}
#[test] fn opaque_output_cannot_be_nested_bound_declaratively() {}
#[test] fn contract_fidelity_downgrade_is_diff_visible() {}
#[test] fn trusted_extension_assurance_is_derived_not_source_asserted() {}
#[test] fn unmanaged_escape_requires_source_and_deployment_approval() {}
#[test] fn managed_to_unmanaged_change_is_security_significant_diff() {}
#[test] fn unmanaged_escape_receives_no_secret_slot_implicitly() {}
```

## 21.3 Do not test trivia

Avoid private Vue refs, Reqwest internals, parser internal node shapes and large
opaque snapshots as primary assertions.

## 21.4 Accessibility

axe-core floor plus explicit keyboard navigation for the acceptance fixture.

---

# 22. PoC implementation plan

The schedule is gated, not a promise to build everything. v0.16 deliberately
places two product gates before substantial managed-runtime investment.

| Week | Increment | Deliverable / gate |
| --- | --- | --- |
| 0 | Bootstrap + pre-registration | repo, CI, ADRs, B-full baseline, instrumentation harness, dev/hold-out task manifest, statistical plan |
| 1 | Source + compiler slice | JSONC adapter, stable spans/diagnostics, one page, deterministic IR skeleton |
| 2 | Renderer slice | `JsonRenderAdapter`, Vue host, built-in text/form/table skeleton |
| 3 | OpenAPI read path | `oas3` strict visitor, one real GET, route/binding/list projection; renderer-only mechanism control wired |
| **4** | **G0 measurement** | B-full vs uiko mini repo-twin + renderer-only control + small TypeScript-spec authoring probe; GO/NO-GO published |
| **5** | **G1 semantic-review gate** | semantic diff completeness + seeded review pilot over compiled plans; GO/NO-GO published |
| 6 | Mutation/runtime core, only if G0+G1 GO | authz, CSRF category, validation, SQLite journal/audit intent, one mutation |
| 7 | Client/deployment identity | JCS vectors, expected-digest build pipeline, `ClientContractManifest`, `explain`; one Hybrid TypeScript/React `RuntimeClient` call |
| 8 | Extension + security seams | synthetic non-HTTP adapter; Web Component ABI/SRI/`allowedActions`; redirect/CSP/Class C/fidelity/Managed→Unmanaged tests |
| 9 | Dress rehearsal + freeze | poisoned/crash fixtures, dev harness, classifier agreement, final reviewer-study dry run, engineering freeze |
| 10 | Hold-out measurement | unseal, B-full vs uiko runs, clustered analysis, expressiveness + final review study, publication |

If G0 is NO-GO, weeks 5–10 are not the default plan. If G0 passes but G1
fails, the compiler may still justify a narrower generation/validation product,
but the review/trust-runtime thesis is not assumed.

---

# 23. Acceptance fixture — `support-console`

## 23.1 Capabilities

| Capability | Behavior |
| --- | --- |
| Customer list | cursor pagination + table |
| Dashboard table | page pagination to prove heterogeneous list binding |
| Customer detail | route-bound GET + standard display components |
| Edit customer | mutation with server validation and declared upstream idempotency variant |
| Unsafe mutation fixture | same mutation without upstream idempotency support for indeterminate-crash test |
| Runtime state | loading, validation error, timeout, upstream failure, rate limit, indeterminate |
| Conditional UI | simple visibility condition; explicitly not authorization |
| Custom UI | interactive graph via Custom Element + `allowedActions` |
| Operability | every stable ID explainable |
| Deployment | reviewed revision + expected digest enforced |
| Hybrid surface | plain TypeScript/React client through `RuntimeClient` |
| Contract fidelity | typed, partial and opaque provider fixtures |
| Assurance | Managed, Trusted Extension and Unmanaged seeded paths |

## 23.2 Acceptance changes

### A — ordinary field

Add `lifetimeValue` to Customer Detail; API already exposes it.

Expected: feature source only.

### B — existing action

Add a button invoking an already imported operation.

Expected: feature source only; compiler validates action/input/scopes.

### C — list shape change

Change one fixture endpoint from page to cursor pagination.

Expected: integration binding + feature behavior; no hand-written HTTP/query
plumbing.

### D — complex visualization

Add a graph not reasonably representable by built-ins.

Expected: component implementation + contract + feature usage; compiler/runtime
core unchanged.

### E — framework swap

Replace the graph implementation with another framework while preserving the
contract.

Expected: uiko feature source and compiler semantics unchanged.

### F — adversarial payload

API returns strings resembling instructions/markup/script payloads.

Expected: built-ins render inert text; managed logical wiring does not mutate
itself based on payload.

### G — crash after possible upstream commit

Force process failure after request transmission but before local completion
persistence.

Expected:

- with propagated upstream idempotency support: safe reconciliation/retry per
  declared policy;
- without it: journal becomes `INDETERMINATE`, no blind re-execution.

### H — Hybrid client

Invoke `customers.updateCustomer` from a plain React/TypeScript fixture using
`RuntimeClient`, with no `UiManifest` consumption.

Expected: same logical ID, authz, validation, idempotency and audit behavior as
the Full renderer path.

### I — contract fidelity downgrade

Replace one typed external response contract with a reviewed partial/opaque
variant.

Expected: semantic diff names the downgrade; nested declarative binding into
unknown structure fails; pass-through/custom consumption remains possible.

### J — unmanaged escape

Change one custom integration path from managed invocation to an explicitly
unmanaged path.

Expected: compile/review requires explicit escape declaration, deployment
policy must approve, `diff` marks a security-significant downgrade, `explain`
states lost guarantees, and no uiko secret slot is delegated implicitly.

## 23.3 Recorded measurements

Per task/run:

- task ID and provenance;
- model/config/seed actually used;
- completion outcome;
- files and semantic areas touched;
- final accepted application-owned changed tokens/bytes and lines;
- cumulative inserted/replaced edit tokens/bytes across the run (including discarded attempts);
- generated/build artifacts excluded by a frozen path policy;
- model input/output token usage when the provider reports complete telemetry;
- repository read/search/tool-call counts as context-work indicators;
- assurance/fidelity changes touched by the task;
- validation/repair iterations by pre-registered category;
- browser iterations;
- compiler/runtime internals opened or modified;
- custom component escape used and why;
- tokens/context/wall-clock where reliably measurable.

---
# 24. Experiment and falsification plan

## 24.1 Principle

The experiment exists to answer a product question, not to decorate the README.
The primary comparison is **uiko vs a strong conventional typed frontend**,
but v0.16 decomposes the claim explicitly:

1. does the agent have to invent materially less application implementation?
2. does whole-plan compilation reduce repair work beyond a renderer-only JSON effect?
3. can humans review consequential behavior more effectively?
4. only then, does the managed runtime justify its adoption cost?

Iterations and tool calls are nested observations. **The task is the primary
inferential cluster.** Repeating one task does not create independent tasks.

## 24.2 G0 mini repo-twin and mechanism probes

Before runtime investment, run 5–6 **development tasks** in both primary
conditions:

- **B-full:** current conventional frontend baseline frozen at Week 0;
- **C:** uiko compiler/renderer slice.

Instrument every run for accepted and cumulative edit payload, files touched,
model usage where complete, repository-read/tool-call work, validation repairs
and browser actions. G0 is diagnostic rather than publication-grade inference.
Publish raw task results and decide GO/NO-GO using §4.1.

Two small mechanism probes run on at least three of those tasks:

- **R-render-only:** direct json-render UI spec/catalog plus conventional
  TypeScript query/mutation/application plumbing. It tests whether "JSON UI"
  alone explains the observed reduction.
- **T-authoring:** a minimal TypeScript object/spec facade lowered to the same
  uiko source DTO. It tests whether JSONC is helping the semantic model or
  merely creating a proprietary authoring tax.

R and T are diagnostic controls, not final experimental arms and not production
support commitments. No hold-out task may be used to tune compiler semantics,
renderer mappings, authoring choices or catalog contents.

## 24.3 Final arms

### B-full — strong conventional baseline

Week-0 exact versions are frozen, but the conceptual stack is:

```text
Vite + React + TypeScript strict
TanStack Query
zod or equivalent runtime schema validation
OpenAPI-generated client/types
mature component library
ESLint + tsc
Playwright-capable acceptance/browser loop
```

The baseline MAY be adjusted before pre-registration if a materially stronger
2026 default exists; after freeze, no arm gets asymmetrical upgrades.

### C — uiko

```text
primary uiko authoring source (JSONC unless G0 authoring decision changes before freeze)
uiko compiler + stable diagnostics
JsonRenderAdapter / built-in catalog
managed runtime for final hold-out measurement
same acceptance task statement and equivalent browser capability
```

No naive Arm A, B-CLI arm, renderer-only arm or TypeScript-authoring arm enters
the final hold-out. They add session cost without answering the purchasing
question. R/T are G0 mechanism probes only; final inference remains B-full vs C.

## 24.4 Task set and hold-out

Target:

- **≥20 tasks total**;
- 8 development/tuning tasks;
- **≥12 hold-out tasks**;
- ≥30% adapted from real OSS issues/PRs or externally authored realistic
  requirements, with license/provenance recorded;
- remaining tasks authored by the team and reviewed by a non-author;
- task manifest, expected acceptance behavior and classification examples
  committed before measurement.

Hold-out tasks are stored in a digest-pinned sealed location/revision and are
not opened by implementers before the measurement window. Any breach is
recorded and invalidates claims that rely on the hold-out.

## 24.5 Runs and randomness

Each hold-out task is run under at least **two pre-registered seeds/configured
replicates per arm** when the model/provider actually supports meaningful seed
variation.

If the provider is deterministic at the chosen settings, identical reruns are
not treated as independent evidence. Repetitions measure operational variance;
they do not multiply task-level `n`.

Record actual model version/provider parameters because hosted model behavior
can drift even when a nominal model name is unchanged.

## 24.6 Iteration classification

Each failed/repaired iteration receives one primary category based on what the
error **is**, not which technology it touches:

```text
LOCAL
  one expression/declaration/schema use within one layer

WIRING
  connection between layers
  e.g. wrong route param → request input, wrong action mapping,
  missing invalidation, wrong list projection path

COHERENCE
  whole-plan/contract invariant
  e.g. authorization coverage, component contract/event/allowedActions

VISUAL_FIT
  static plan is valid but rendered behavior/appearance fails acceptance

ENVIRONMENT
  tooling/provider/build failure not attributable to application semantics
```

Ambiguity is adjudicated conservatively **against uiko's claimed
mechanism**. Two classifiers rehearse on dev transcripts before the hold-out;
Cohen's κ is reported. If agreement is poor, refine examples before freeze,
not during measurement.

## 24.7 Primary hypotheses

### H0 — agent-authored implementation burden

Two durable per-task metrics are computed from the complete edit trace using a
frozen path/tokenization policy:

```text
accepted_change_tokens
  lexical tokens inserted/replaced in application-owned source in the final
  accepted diff; generated artifacts, lockfiles and build output excluded

cumulative_authored_edit_tokens
  lexical tokens inserted/replaced across every file-edit operation during the
  run, including superseded/failed attempts
```

Report files touched and changed bytes/lines alongside them. Provider-reported
model input/output tokens are also reported when telemetry is complete across
both arms, but they are not the only durable metric because provider accounting
can change.

Headline H0 success requires, versus B-full:

- paired task-level point estimate **≥40% fewer accepted change tokens**;
- paired task-level point estimate **≥30% fewer cumulative authored edit tokens**;
- 95% task-clustered paired bootstrap interval excludes zero improvement for at
  least the accepted-change metric; and
- completion in C is not more than 10 percentage points worse than B-full.

A win caused by generated hidden glue, compiler/core edits or materially more
browser repair does not count; those surfaces are reported separately.

### H1 — cross-layer repair

Primary metric per task:

```text
WIRING + COHERENCE repair iterations until accepted completion
```

Primary effect: paired mean task-level reduction from B-full to C.

Headline success requires:

- point estimate **≥30% fewer** WIRING+COHERENCE repair iterations; and
- 95% **task-clustered paired bootstrap** confidence interval excludes zero
  improvement; and
- task completion in C is not more than 10 percentage points worse than B-full.

If the CI is wide because the effect/task count is small, report that. Do not
resample raw iterations as though they were independent.

### H2 — edit locality

Report per task:

- files touched;
- semantic areas touched;
- source lines changed as descriptive context;
- whether core/compiler/runtime internals were modified.

Target: median ordinary-task file count at least 40% lower in C, with no
increase in core modifications.

### H3 — visual/fitness displacement

uiko must not obtain generation/static-check gains by pushing equivalent work
into the browser loop. Report VISUAL_FIT iterations and total acceptance-loop
actions.

No positive claim if H0/H1 improve but visual/fitness work increases enough to
offset the operational advantage materially.

### H4 — mechanism is more than JSON rendering

G0's R-render-only probe is descriptive because it uses development tasks, but
its result constrains the product claim. If direct json-render plus conventional
plumbing captures nearly all H0/H1 improvement, uiko MUST narrow its thesis
to the remaining compiler/review/runtime value rather than claiming the
generation reduction as proprietary.

## 24.8 Secondary operational metrics

Record, when trustworthy:

- provider-reported input/output tokens and cache/context metrics;
- repository read/search operations and edit-tool calls;
- wall-clock to accepted completion;
- compiler invocations;
- browser invocations;
- failure/recovery reasons;
- custom component escape use.

These are secondary. Provider pricing/token accounting changes quickly and is
not part of the durable architecture claim.

## 24.9 Expressiveness and escape budget

On hold-out tasks only, classify each requested UI capability as:

```text
BUILT_IN
CUSTOM_COMPONENT
UNSUPPORTED
```

Primary expressiveness measure is agent-iteration-weighted built-in coverage.
Target gate: **≥70%** of target operational UI work handled without a new
custom component.

Every custom escape logs:

- feature/task;
- built-ins considered;
- reason for escape;
- implementation framework/library;
- whether the same missing capability recurred elsewhere.

Repeated escapes are evidence for catalog growth. One exotic visualization is
not.

## 24.10 Local-model experiment

Optional secondary experiment after the main harness is stable. Freeze local
model, quantization and hardware. Compare a subset of ordinary tasks under B
and C.

Report completion and repair iterations. Do not publish a broad claim that
uiko "makes weak models good at frontend" unless the result actually
supports it.

## 24.11 Review-efficacy study

G1 (§4.2) runs an early, smaller pilot before runtime investment. The final
hold-out study below is the publication-grade test and is not replaced by G1.

The semantic diff claim has two separate obligations:

1. **completeness:** every plan semantic change appears in the diff (§16.3),
   tested mechanically;
2. **human detection:** reviewers notice meaningful risky changes.

Human study:

- ≥24 review diffs;
- roughly half clean and half containing one seeded contract/security-relevant
  change;
- seeded changes include authorization widening, invalidation removal,
  operation remap, `allowedActions` widening, unsafe mutation-policy change or
  provenance/contract drift;
- ≥6 reviewers where feasible; ≥2 not on the implementation team;
- randomized order, reviewers informed only that some diffs contain planted
  issues.

Report:

- sensitivity (seeded issues detected);
- specificity / clean diffs correctly accepted;
- per-reviewer and per-diff distributions;
- uncertainty using a two-way reviewer/diff clustered bootstrap or a suitable
  mixed-effects/binomial model;
- review-time distribution **descriptively only**.

No `<15 seconds = rubber stamp` rule. Review duration is context, not proof of
review quality.

Product claim "semantic diff is an effective review surface" is falsified for
this PoC if seeded-issue sensitivity is <70% or if the study reveals a repeated
semantic-impact class that reviewers cannot reliably recognize from the diff.

## 24.12 Security/adversarial scenarios

Required scenarios:

- poisoned response text remains inert on built-in paths;
- undeclared managed operation/binding fails closed;
- disallowed redirect is not followed;
- custom bundle substitution is refused;
- normal custom event wiring outside `allowedActions` is rejected;
- hostile same-origin component demonstrates that bridge restrictions are
  bypassable, confirming the documented Class B residual rather than pretending
  it was eliminated;
- CSRF regression for cookie session;
- audit/journal durable-write failure prevents mutation execution;
- crash-window tests produce safe replay only with declared upstream
  idempotency, otherwise `INDETERMINATE`.

## 24.13 Competitive sanity checks

Controlled inference remains B-full vs uiko. Product competitors are not
shoehorned into statistically symmetric arms when their hosted/platform
semantics make that comparison artificial. Before publication, maintain a
current capability matrix for:

- **Retool / Superblocks / Windmill:** AI-built app workflow, Git/code ownership,
  governance, self/private deployment where applicable, version/review model;
- **A2UI / json-render / MCP Apps:** primitives uiko should reuse or adapt
  instead of rebuilding;
- **Wasp:** spec-driven authoring precedent and the custom-DSL → TypeScript
  migration as an authoring-risk signal;
- **Encore / Nitric:** application-model/provider-adapter precedents.

These comparisons constrain positioning, not the compiler's semantic design.
Claims are dated and re-verified under §33.

## 24.14 Freeze and publication

Before hold-out unseal:

- freeze arms/tool versions;
- freeze catalog and compiler semantics;
- freeze task acceptance criteria;
- freeze statistical code/thresholds;
- run classifier and reviewer-study dress rehearsal on development material;
- record environment/tool versions.

Publish raw harness logs, task-level aggregates, deviations and negative
results subject to any licensing/privacy constraints.

---

# 25. Quality gates

## Rust

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
cargo audit
cargo deny check
```

## Web

Representative gates:

```text
typecheck
lint
unit/component tests
axe-core acceptance checks
keyboard Playwright E2E
production build
```

## Determinism / security / artifact gates

CI additionally verifies:

- RFC 8785 vector suite;
- independent JCS property corpus;
- byte-identical rebuild for frozen fixture inputs;
- JSONC patch comment survival;
- semantic diff completeness;
- expected reviewed plan digest build check;
- runtime plan absent from static assets;
- component digest verification;
- extension registry/conformance regressions;
- unregistered extension fail-closed regression;
- extension normalized-fragment determinism/diff visibility;
- redirect allowlist regression;
- CSRF regression;
- audit/journal fail-closed mutation behavior;
- indeterminate mutation crash window;
- CSP header baseline for reference deployment.

---

# 26. Distribution and product surface

## 26.1 CLI

Minimum useful commands:

```text
uiko validate [--format json]
uiko set ...
uiko patch ...
uiko diff [--format json]
uiko explain <symbol> [--json]
uiko hash
uiko dev
uiko build [--expect-plan <digest>]
uiko client generate --target ts
```

## 26.2 MCP/agent adapter

A thin MCP server or coding-agent integration may wrap the CLI's structured
interfaces. It MUST NOT contain independent compiler semantics.

The agent-facing contract is stable diagnostics + structured edit/diff/explain
operations, not terminal scraping.

Extension tooling may later add commands such as `uiko extension check` or
`uiko extension inspect`, but dynamic remote installation is not required
for the product to be extensible.

## 26.3 Four adoption profiles

| Profile | Adopt | Value |
| --- | --- | --- |
| **Validation-only** | compiler/CLI + project source/contracts | deterministic repair/review tooling; no uiko production runtime |
| **Full** | declarative UI + renderer + managed runtime | maximum compiler locality and managed-path coverage |
| **Hybrid** | existing/custom frontend + `RuntimeClient` + managed runtime | adopt authz/audit/capability plane without rewriting the UI; migrate route-by-route if useful |
| **Runtime-only** | managed runtime + client protocol, no uiko UI tree | use uiko as a reviewed capability plane for any frontend/client |

Validation-only is a valid product outcome. Full, Hybrid and Runtime-only are
not different backends: they share `ClientContractManifest`, logical operation
identity and server controls.

## 26.4 Client surface

PoC ships a small TypeScript `RuntimeClient` capable of:

```text
loadClientContract()
query(operationId, input)
command(operationId, input, idempotencyKey)
normalize uiko result/error categories
```

It contains no independent policy semantics. Generated typed wrappers may be
created from `ClientContractManifest` (`uiko client generate --target ts`)
when useful, but the runtime protocol remains usable without code generation.
Future SDKs may target other languages.

## 26.5 Product claims after v0.16

Claims allowed only when the relevant proof obligation passes:

- **generation-burden claim:** materially less accepted and cumulative agent-authored application implementation on measured tasks;
- **compiler-loop claim:** fewer cross-layer repair iterations on measured
  declarative-UI tasks;
- **locality claim:** ordinary declarative changes touch less application surface;
- **review claim:** semantic diff is complete and seeded issues are detected at
  the measured rate;
- **managed-path claim:** declarative Class A mistakes fail closed under the
  tested scope;
- **hybrid claim:** existing frontends can invoke the same managed capability
  plane without adopting `UiManifest`;
- **assurance claim:** the product distinguishes Managed, Trusted Extension and
  Unmanaged paths without silently upgrading guarantees;
- **contract-fidelity claim:** partial/opaque systems can be connected while
  unsupported static guarantees are explicitly withheld;
- **custom-component claim:** framework-neutral contract boundary works for the
  demonstrated implementations;
- **extension-seam claim:** a demonstrated non-HTTP capability adapter enters
  through the same core contract/executor boundary without vendor-specific
  compiler/source semantics;
- **deployment identity claim:** the runtime plan matches the reviewed revision
  and expected digest.

Do not publish “secure autonomous frontend”, “anything is secure because it is
behind uiko”, or hostile-code-isolation language for the PoC.

## 26.6 Initial ICP

Initial focus remains teams already using coding agents on operational/internal
UIs over existing systems, where UI churn, wiring and review cost are visible.

The adoption surface is deliberately broader than the initial ICP: an existing
React/Vue/Angular application can start Hybrid; a proprietary desktop/client UI
can use Runtime-only; ERP/data/industrial/proprietary systems can enter through
capability adapters. Product focus does not require architectural lock-in.

---

# 27. Catalog governance

The catalog is a versioned input to compilation.

Each built-in registry entry records at least:

- component kind/version;
- contract/schema digest;
- implementation bundle/version as applicable;
- accessibility expectations;
- provenance/owner.

Catalog changes appear in plan provenance and semantic diff when they alter the
compiled application.

No action-group abstraction in the PoC. `allowedActions` remain explicit. The
escape log is the evidence source for any future grouping/catalog expansion.

---

# 28. Bet ladder

| Rung | Product | Evidence to justify the rung | Trigger to climb |
| --- | --- | --- | --- |
| **0** | compiler/validation experiment | G0 shows useful wiring/locality effect | build final PoC |
| **1** | validation-only + Full reference app | final repo-twin supports H1/H2 and users actually try it | repeated use on real projects |
| **2** | Hybrid/Runtime-only managed capability plane | teams value authz/audit/review identity without requiring UI rewrite | production adoption + operational feedback |
| **3** | broader provider ecosystem | real demand for language-neutral providers, jobs/streams, additional contract importers | repeated integration requirements |
| **4** | harder platform capabilities | evidence for multi-replica state, sandboxed browser code or plan hot-swap | concrete operational requirements |

The desired failure mode is a smaller useful tool, not a universal platform
built before anyone demonstrates the need.

---

# 29. Non-goals for the PoC / v1

- general backend generation;
- workflow engine;
- visual low-code editor;
- arbitrary programming inside uiko JSONC;
- dynamic marketplace/plugin installation or runtime remote-code loading;
- first-party adapters for every external technology;
- forbidding arbitrary application behavior merely because uiko cannot manage it;
- pretending Unmanaged Escapes inherit uiko runtime guarantees;
- distributed multi-replica runtime semantics;
- globally consistent distributed rate limiting;
- hostile-code sandbox for custom components;
- uiko-rendered unauthenticated login page;
- realtime/AsyncAPI execution;
- hot-swappable signed external RuntimePlan;
- automated rebuild attestation that bypasses human custom-component review;
- action groups/wildcard component permissions;
- exactly-once guarantee across arbitrary external systems;
- solution to operator social engineering.

---

# 30. Decision record — v0.16 baseline

| ID | Decision | Chosen | Rejected / deferred | Reason |
| --- | --- | --- | --- | --- |
| D1 | Core semantic implementation | Rust 2024 | JS/TS compiler core | deterministic compiler/runtime boundary and one production server language |
| D2 | OpenAPI parser | `oas3` + strict uiko visitor | own OpenAPI parser | parsing is not the product; supported semantics are |
| D3 | Authoring syntax | JSONC primary implementation + G0 TypeScript-spec probe | YAML bake-off; permanent JSONC identity before evidence | keep one buildable path while explicitly testing proprietary-syntax risk |
| D4 | JSONC CST | Rust `jsonc-parser` CST | hand-rolled CST; generic rowan layer | purpose-built parser/manipulator already provides required edit boundary |
| D5 | Renderer substrate | json-render behind adapter | own generic renderer first | reuse catalog/render machinery while preserving replacement boundary |
| D6 | First host | Vue 3 | framework semantics in IR | implementation choice only |
| D7 | Complex component ABI | Custom Elements | compiler-specific React/Vue/Svelte semantics | framework-neutral browser contract |
| D8 | Component permissions | explicit `allowedActions` | action groups in PoC | smaller review surface and less governance machinery |
| D9 | Custom bundle changes | human review for any digest/contract change | attestation automation | optimize after real rebuild volume exists |
| D10 | Canonical plan identity | RFC 8785 JCS + digest | ordinary serde JSON hash | cross-platform canonical bytes |
| D11 | Plan storage | embedded only for PoC | signed external/hot swap | one deployment path; remove premature key/signature lifecycle |
| D12 | Review binding | protected `(sourceRevision, expectedPlanDigest)` + deterministic CI build | expected digest stored only in agent-writable repo | independent review-to-build identity |
| D13 | Integration config | separate `IntegrationConfigProvider` and `SecretProvider` | one overloaded secret/config port | endpoint policy and credential custody are different responsibilities |
| D14 | Mutation state | durable SQLite journal | in-memory idempotency map | crash semantics are part of correctness |
| D15 | Mutation guarantee | completed replay + safe upstream-key recovery + indeterminate state | exactly-once claim | arbitrary upstream APIs cannot provide universal exactly-once |
| D16 | Audit | durable intent before execute + durable outcome | tracing-only mutation audit | fail closed on missing evidence |
| D17 | Runtime topology | single instance | implicit multi-replica support | shared idempotency/rate-limit semantics require a different storage architecture |
| D18 | Class B custom code | trusted same-origin in PoC | pretending CSP/SRI/allowlists sandbox it | browser same-origin physics |
| D19 | CSP | explicit defense-in-depth directives incl. `form-action` | `default-src` treated as universal egress control | navigation directives have different fallback behavior || D20 | Experiment arms | B-full vs uiko | naive arm + CLI sub-arm | answer the purchasing/product question with fewer sessions |
| D21 | Inference unit | task-clustered paired analysis | raw iteration bootstrap | avoid pseudoreplication |
| D22 | Review-time metric | descriptive | `<15s = rubber stamp` falsification | time alone is not review correctness |
| D23 | Runtime investment gate | G0 at week 4 | build full runtime before core validation | kill bad bets early |
| D24 | General extensibility model | uiko-owned capability contracts + adapters | protocol/vendor semantics in source/compiler | constrained core with open boundaries |
| D25 | First external-system adapter | OpenAPI/HTTP | HTTP as core semantic model | realistic PoC path without architectural lock-in |
| D26 | Extension approval | explicit operator/build `ExtensionRegistry` | source-driven dynamic plugin loading | agent source may select capabilities, not executable supply chain |
| D27 | Server extension trust | in-process adapters are Class C trusted code | pretending trait/interface conformance is sandboxing | process authority cannot be hidden by an interface |
| D28 | Third-party adapter boundary | future language-neutral out-of-process capability provider | require all custom adapters to be Rust/in-process forever | permit Java/Python/Go/etc. and narrower isolation without core changes |
| D29 | Capability kinds | `query` + `command` PoC; `job`/`stream` explicit later | model every interaction as HTTP request/response | lifecycle semantics should be generic and honest |
| D30 | Extension reviewability | deterministic schema-validated plan fragment + explain/diff projection | opaque behavior-changing adapter bytes | review-to-production identity must survive extensibility |
| D31 | Adoption architecture | Full + Hybrid + Runtime-only + Validation-only | declarative UI as mandatory entry point | existing systems must be able to adopt incrementally |
| D32 | Public capability boundary | `ClientContractManifest` post-auth | `UiManifest` as the only public contract | non-uiko frontends need the same typed logical operations without renderer coupling |
| D33 | Managed client protocol | one `RuntimeClient` path for Full/Hybrid/Runtime-only | separate “raw API” for external frontends | one authorization/validation/audit semantics |
| D34 | Assurance model | Managed / Trusted Extension / Unmanaged Escape | binary “supported/unsupported” or pretending all extensions are equally managed | flexibility must not hide where guarantees stop |
| D35 | Contract fidelity | explicit `typed` / `partial` / `opaque` | reject every imperfect system or infer schemas from payloads | connect legacy systems without manufacturing certainty |
| D36 | Unmanaged escape policy | explicit source intent + deployment approval + diff/explain visibility | silent direct-fetch bypass or absolute prohibition | preserve user freedom while protecting review semantics |
| D37 | Assurance derivation | compiler/registry/runtime derived | source-authored `assurance: managed` claims | an agent cannot self-certify its own trust level |
| D38 | Opaque binding semantics | pass-through allowed; nested declarative binding forbidden | dynamic unchecked field traversal | unknown structure must remain unknown until explicitly contracted |
| D39 | Product value order | reduce invention → compile behavior → review semantics → govern execution | lead with verification/runtime alone | model the actual agent + human cost chain and gate runtime investment |
| D40 | G0 mechanism control | small direct-json-render control on dev tasks | treat any JSON-vs-React reduction as uiko value | isolate renderer compression from whole-plan compiler value |
| D41 | Review investment gate | G1 seeded semantic-diff pilot before managed runtime | build runtime first and test review last | review bandwidth is a product thesis, not a documentation feature |
| D42 | Agent-effort measurement | accepted + cumulative authored edit tokens, plus telemetry/context indicators | final LOC or vendor token price alone | measure both shipped surface and failed/replaced generation effort |

---

# 31. Changelog v0.14 → v0.15

| # | Change | Why |
| --- | --- | --- |
| V15-1 | promoted **agent generation burden** to the first product problem | uiko should reduce behavior the agent must invent, not only validate it afterward |
| V15-2 | reframed the thesis as **reduce invention → compile behavior → review semantics → govern execution** | makes the causal/product chain explicit and independently falsifiable |
| V15-3 | split pre-runtime validation into **G0** (generation + compiler) and **G1** (semantic review) | runtime adoption cost is unjustified if either core value fails |
| V15-4 | added accepted-change and cumulative-authored-edit token metrics | final LOC misses failed/replaced agent generation and can be gamed by syntax density |
| V15-5 | added a small **renderer-only json-render mechanism control** | JSON/spec UI is already established; uiko must prove value beyond renderer compression |
| V15-6 | reopened authoring identity without reopening implementation scope: JSONC remains primary, with a small TypeScript-spec probe | Wasp's 2026 DSL→TypeScript migration is a concrete warning against confusing custom syntax with product value |
| V15-7 | upgraded Retool, Superblocks and Windmill to explicit product-competitor class | governance + AI-built apps + Git/deployment are not unique claims |
| V15-8 | classified A2UI/json-render/MCP Apps as reusable primitives/adjacent standards rather than ground to recreate | concentrate implementation on compiler/review/runtime semantics |
| V15-9 | added Wasp and Encore/Nitric as architectural precedents/risk signals | spec-driven compiler and provider-adapter patterns already exist; architecture alone is not a moat |
| V15-10 | made semantic review a pre-runtime product gate plus final publication-grade study | review bandwidth is one of the core reasons for uiko to exist |
| V15-11 | added competitive sanity checks to the experiment protocol | prevent a technically successful PoC from being positioned against an obsolete market baseline |

v0.15 intentionally **does not** add another production authoring language, a
new renderer, a marketplace or more runtime features. It changes what must be
proven before those investments are justified.

---

# 32. References and verification anchors

Normative / technical anchors used by this specification:

- RFC 8785 — JSON Canonicalization Scheme: https://www.rfc-editor.org/rfc/rfc8785
- OpenAPI Specification 3.1: https://spec.openapis.org/oas/v3.1.1.html
- OpenAPI 3.1 dialect metadata: https://spec.openapis.org/oas/3.1/dialect/2024-11-10.html
- JSON Schema Draft 2020-12: https://json-schema.org/draft/2020-12
- Rust `jsonc-parser`: https://docs.rs/jsonc-parser/
- `oas3`: https://docs.rs/oas3/
- `jsonschema` crate: https://docs.rs/jsonschema/
- json-render: https://github.com/vercel-labs/json-render
- Google A2UI v0.9 (non-normative market/interop anchor): https://developers.googleblog.com/a2ui-v0-9-generative-ui/
- Wasp TypeScript Spec / retired custom DSL (non-normative authoring precedent): https://wasp.sh/blog/2026/06/15/wasp-typescript-spec
- Windmill deployment/Git/full-code docs (non-normative product baseline): https://www.windmill.dev/docs/advanced/deploy_to_prod
- Retool current app-builder positioning (non-normative product baseline): https://docs.retool.com/
- Superblocks governed AI-app workflow (non-normative product baseline): https://www.superblocks.com/blog/from-claude-prototype-to-production-app
- MDN CSP overview: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy
- MDN `form-action`: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/form-action
- MDN Trusted Types / CSP directives as applicable
- Web Components / Custom Elements and `CustomEvent` platform specifications / MDN
- Axum, Tokio, Reqwest, `thiserror`, `miette`, `schemars`, `ts-rs`
- axe-core and Playwright documentation

Competitive/project references such as A2UI, MCP Apps, json-render, Retool,
Superblocks, Windmill, Wasp, Encore, Nitric, Power Apps, Appsmith, ToolJet,
Budibase and current coding-agent/browser tooling are
**non-normative market references** and must be re-verified before any public
comparison.

---

# 33. Standing re-verification instruction

Before any of the following events:

- Week-0 experiment freeze;
- G0 comparison;
- final hold-out unseal;
- public README/launch post;
- security claim in buyer-facing material;

re-verify:

1. the strongest realistic B-full frontend/agent baseline;
2. relevant current renderer/catalog/agent-UI capabilities, especially json-render, A2UI and MCP Apps;
3. OpenAPI/JSON Schema, client-manifest/runtime protocol and extension-adapter behavior for the exact pinned versions;
4. browser/CSP behavior relied upon by reference deployment;
5. current governed-app platform capabilities (Retool, Superblocks, Windmill) and any competitive traction/adoption claim;
6. the current Wasp/spec-authoring model before citing it as an authoring precedent.

Architecture decisions may remain stable while ecosystem facts change. Do not
freeze a 2026 market observation into a semantic invariant.

---

# 34. Changelog v0.15 → v0.16

| # | Change | Why |
| --- | --- | --- |
| V16-1 | renamed the product to **uiko** across the complete specification | establish the new product identity consistently before implementation and public use |
| V16-2 | aligned the CLI with the **uiko** product name, including `uiko validate`, `uiko diff`, `uiko explain` and `uiko build` | avoid a split brand between product and tooling |
| V16-3 | aligned product-prefixed source files, `data-uiko-*` attributes and `UIKO...` diagnostics with the new name | remove residual implementation identifiers tied to the previous name |
| V16-4 | architecture, gates, semantics and PoC scope otherwise unchanged from v0.15 | this revision is intentionally a naming migration, not an architectural redesign |

---

**End of v0.16.**