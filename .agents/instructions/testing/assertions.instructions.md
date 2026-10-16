---
description: "Use when writing test assertions or reviewing test quality. Covers assertion patterns and regression test focus."
---

## Assertions

- Prefer assertions that check behavior, not incidental implementation details.
- Keep regression tests focused on the bug or contract being changed.
- Assert collection order only when the API contract guarantees order. For an
	order-sensitive contract, assert the complete ordered result with stable,
	ordered fixtures; never use random v4 UUID fixtures when order is under test.
	Otherwise assert membership, cardinality, or another order-independent
	property. This prevents the `BTreeSet` board-aggregation first-seen-order
	defect from being hidden by incidental fixture ordering.
- For who is authorized to author or edit test files under the Planner/Worker split, see [split-responsibility-testing.instructions.md](split-responsibility-testing.instructions.md).
