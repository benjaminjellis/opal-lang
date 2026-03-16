# Row Polymorphism Plan (Mond)

## Why This Exists

Mond currently has nominal record types, but unqualified field access syntax (`:field`) looks structural. The compiler can typecheck and codegen field access incorrectly when multiple records share a field name. This document plans a long-term fix toward row-polymorphic field operations.

## Current Constraints

- Type representation is HM over `Type::{Fun, Con, Var}` only.
- Field access typing depends on globally registered accessors and fallback by plain `:field` symbol.
- Codegen lowers field access by field name -> tuple index (`field_indices`), not by resolved record type.
- Record runtime representation is tuple-based (`{tag, field1, field2, ...}`), and field order is per-record declaration.

Implication: true structural row polymorphism cannot be added safely without also addressing evidence/codegen for field lookup.

## Decision Point

Two viable paths:

1. Constraint-based row polymorphism over nominal records (recommended first).
2. Full structural row types (records as rows in the core type system).

Recommended ordering: implement (1) first for correctness + ergonomic polymorphic field access; evaluate (2) later only if structural records are a product goal.

---

## Phase 0: Correctness Foundation (Required for Both Paths)

### Goals

- Remove field-name-global ambiguity from typecheck and codegen.
- Make codegen depend on resolved record type, not just field name.

### Work

1. Introduce record-qualified field index map:
   - from `HashMap<String, usize>`
   - to `HashMap<(String, String), usize>` (`(record_name, field_name) -> index`)
2. Carry resolved field-access metadata from typecheck to codegen.
   - Option A: typed AST annotation pass (`ExprId -> ResolvedFieldAccess`).
   - Option B: lower `:field` into explicit accessor calls before codegen.
3. Ban ambiguous plain access during this phase unless resolved by concrete record type.
4. Add regression tests for:
   - same field across two records with different index positions
   - imported modules defining overlapping field names

### Exit Criteria

- No field access uses fallback index by name only.
- Previous `ContinuePayload` vs `Initialised` misinference class is impossible.

---

## Path A (Recommended): Constraint-Based Row Polymorphism on Nominal Records

### Core Idea

Represent field polymorphism as predicates (qualified types), e.g.:

`HasField "selector" r a => r -> a`

instead of immediately committing `r` to one nominal record.

### Phase A1: Type System Extension

1. Extend `Scheme` to carry predicates:
   - `Scheme { vars, preds, ty }`
2. Add predicate enum:
   - `HasField { label: String, record_ty: Type, field_ty: Type }`
3. Inference changes:
   - Field access emits `HasField(label, r, a)`.
   - Record update emits `HasField` constraints for updated fields.
4. Generalization/instantiation must preserve and freshen predicates.

### Phase A2: Constraint Solver + Instance Environment

1. Build instance table from record declarations:
   - one instance per `(record_name, field_name)`.
2. Constraint solving behavior:
   - If `record_ty` is concrete nominal record, discharge immediately.
   - If `record_ty` stays polymorphic, keep constraint in scheme.
3. Add ambiguity diagnostics when constraints are unsatisfied.

### Phase A3: Evidence Passing in IR

1. Introduce implicit dictionary parameters for retained predicates.
2. Monomorphized call sites pass evidence from instance table.
3. Field access IR uses evidence (index/extractor), not global field map.

### Phase A4: Surface UX + LSP

1. Update `type_display` to render qualified types.
2. LSP hover/completions show constraints.
3. Improve diagnostics:
   - missing `HasField`
   - ambiguous/unsatisfied field constraints

### Path A Exit Criteria

- `(:selector x)` is valid in polymorphic helpers when constrained by `HasField`.
- Calls resolve against any nominal record that has `:selector`.
- No runtime field-index mismatches.

---

## Path B (Optional Later): Full Structural Row Types

### Core Idea

Add row kinds directly to types:

- `Record { l1: t1, l2: t2 | r }`
- row variables and row unification
- lacks constraints for update/remove semantics

### Major Impacts

- `Type` gains row constructors and row-unify algorithm.
- Pattern matching and update typing shift to structural rules.
- Runtime representation likely needs maps or uniform field dictionaries.

### Risks

- Large break from nominal-record ergonomics and codegen assumptions.
- Much larger migration/testing surface than Path A.

---

## Milestone Breakdown

1. `M0` (1-2 weeks): Phase 0 correctness refactor + tests.
2. `M1` (1-2 weeks): predicates in typechecker (`Scheme` + inference plumbing).
3. `M2` (1-2 weeks): constraint solving + instance env.
4. `M3` (1-2 weeks): dictionary/evidence passing in IR/codegen.
5. `M4` (1 week): diagnostics, LSP rendering, docs, migration notes.

Total for Path A MVP: about 5-9 weeks depending on refactor depth and test hardening.

## Test Plan Additions

- Unit tests (`mondc/src/typecheck.rs`):
  - polymorphic field accessor inferred with `HasField`
  - unsatisfied field constraint errors
  - imported overlapping field names
- Integration tests (`mondc/src/tests.rs`):
  - cross-module field-polymorphic helper usage
- LSP tests (`mond-lsp/src/tests.rs`):
  - hover displays qualified type constraints

## Open Questions

1. Should qualified constraints be exposed in user syntax now, or only inferred/printed?
2. Do we want explicit type annotations for constrained polymorphic functions in v1?
3. Should record updates be constrained to return same nominal record initially?
4. Do we commit to Path A only, or keep a long-term roadmap to structural rows?

## Immediate Next Slice (Suggested)

Start `M0` with a focused PR:

1. Record-qualified field index plumbing (`project`, `compiler`, `codegen`).
2. Typechecker -> codegen resolved field binding hook.
3. Regression tests for overlapping fields with different positions.

This de-risks everything else and removes the current class of false inferences/miscompilations.
