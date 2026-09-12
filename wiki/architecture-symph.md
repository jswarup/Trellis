# Symph Architecture

## Purpose

`symph` is the reusable kernel and shading-function library. It contains deterministic scalar utilities, element-wise operations suitable for compute dispatch, and graphics-oriented math shared by CPU tests and future shader backends.

## Functional areas

- Hash utilities turn integer seeds into deterministic `uint32_t` values and normalized floats.
- Scalar helpers include a guarded Collatz step-count function. Invalid or overflow-prone inputs return `UINT32_MAX` as a sentinel.
- Element-wise kernels such as `DoubleElem`, `VectorAddElem`, and `CollatzElem` operate on an index and explicit raw arrays. They are shaped so the same operation can run in a CPU loop or in a backend kernel.
- Point-cloud and camera kernels generate point attributes, transform positions, and cull points against six-plane frusta.
- `vertshade.h` defines vector types, camera uniforms, vertex transforms, and point fragment behavior.
- `compshade.h` provides compute-oriented operation definitions and source/label descriptions consumed by `swarm`.

## Execution contract

The element-wise functions receive an element index, pointers to input/output arrays, and explicit element counts. They do not own buffers or schedule work. Bounds and dispatch policy belong to the caller. This makes them usable from direct unit tests, CPU device implementations, or generated GPU kernels.

The graphics functions are also pure or state-light transformations: camera parameters and point data are supplied explicitly, and the result is written to caller-owned output structures or arrays. This keeps the math layer independent of a windowing or rendering framework.

## Data flow

1. A caller chooses a kernel operation and provides contiguous data.
2. A CPU loop or `swarm` backend maps workgroup invocations to element indexes.
3. The Symph function reads the input slice and writes the corresponding output element.
4. Results remain in the caller's buffer and can be read back or passed to another operation.

## Dependencies and consumers

`symph` is a leaf computation layer with standard-library/math dependencies. `swarm` uses its standard operation labels and source descriptions for compute dispatch. Tests call both the scalar functions and shading helpers directly. The library does not depend on `heist` itself, although `swarm` may use Heist to execute batches in parallel.

## Invariants

- Hash results are deterministic for a given input.
- Normalized hash floats are in the half-open interval $[0, 1)$.
- Kernels use explicit sizes and must not write outside their output ranges.
- Invalid Collatz inputs use `UINT32_MAX` rather than wrapping indefinitely.
- Camera and shading functions preserve a backend-neutral, array-oriented calling convention.
