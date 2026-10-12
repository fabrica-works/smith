# Deterministic dependency closure planner

Extend `plan` to schedule only requested targets and their transitive
dependencies. Public types and signatures must remain usable unchanged.
Validate the entire graph first: duplicate node names return
`Duplicate(name)` and missing dependency names return
`MissingDependency { node, dependency }`, even in unreachable nodes.
Validation priority: detect duplicate node names in input order before
checking dependencies in input node/dependency order. After that,
validate requested targets in their input order (`UnknownTarget`).
Repeated dependencies and targets are deduplicated. Ignore cycles outside
the requested closure; a cycle inside it returns `Cycle`, with names
sorted lexicographically of every selected node left unscheduled by Kahn's
algorithm (including nodes blocked downstream of a cycle).
Return each selected node exactly once in dependency-first order. Among
ALL currently ready nodes choose the lexicographically smallest at every
step (not merely sorting each ready batch). Input order must not affect
valid plans. Empty targets returns empty after global graph validation.
Use deterministic bounded work and no recursion; a chain of 10,000 nodes
must not overflow the stack. Add regression tests and run the suite.
