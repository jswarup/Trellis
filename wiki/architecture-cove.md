# Cove Architecture

## Purpose

`cove` contains Trellis's lightweight test framework. It is intentionally small and dependency-light so every subsystem can test public behavior without introducing a larger test runner or framework dependency.

## Main building blocks

- `TestContext` stores run configuration and counters: verbosity, assertion mode, current suite/name, and pass/fail totals.
- `JeevesCase` is an intrusive linked-list node containing a suite name, test name, function pointer, and next pointer.
- `JeevesRunner` is the singleton registry and executor. It appends test cases during registration and runs all or filtered tests.
- `JeevesRegistrar` is an RAII registration object created by the test macro during static initialization.
- `JEEVES_TEST` declares a test function, its static `JeevesCase`, and its static registrar.
- `JEEVES_ASSERT`, `JEEVES_ASSERT_EQ`, and `JEEVES_ASSERT_NE` update context counters and optionally print diagnostics without aborting the test body.

## Registration and execution flow

1. A translation unit expands `JEEVES_TEST`.
2. Static initialization constructs a `JeevesCase` and `JeevesRegistrar`.
3. The registrar inserts the case into `JeevesRunner`'s intrusive list.
4. The console entry point invokes `RunAll` with a filter, verbosity, and assertion setting.
5. The runner sets the current suite/name, invokes the test function with its context, and aggregates results.

Because registration is intrusive, test discovery requires no central list and no dynamic test factory. The test executable only needs to link the translation units containing the tests.

## Assertion behavior

Assertions are non-throwing checks. Each assertion increments the total count. When enabled, it increments either pass or fail and emits a diagnostic according to verbosity. When disabled, the test body still executes but the condition is not evaluated by the macro's assertion branch.

## Dependencies and consumers

`cove` uses the C++ standard library for streams and string comparison support. It is consumed by tests in `silo`, `heist`, `rube`, `swarm`, and `symph`; it is not a runtime dependency of those libraries.

## Invariants

- Every registered test has a stable suite/name/function triple.
- Registration preserves a simple linked-list traversal order.
- A failed assertion records failure but does not terminate the current test.
- Test diagnostics identify the source file and assertion line.
