# Swarm Architecture

## Purpose

`swarm` is Trellis's backend-neutral compute execution API. It presents buffers, compiled kernels, devices, workgroup dimensions, and structured errors through interfaces, then supplies a working CPU backend and placeholders for GPU backends.

## Main building blocks

- `BackendKind` identifies CPU, Rust GPU, and Cuda/Oxide backends.
- `BufferUsage` describes how a compute buffer may be used, such as storage and read/write access.
- `WorkgroupDim` describes the dispatch shape, including linear work.
- `IComputeBuffer` is the byte-oriented buffer interface: size, label, write, and read.
- `IComputeKernel` is the compiled-kernel interface: name and backend identity.
- `IComputeDevice` owns buffer creation, kernel compilation, dispatch, and synchronization.
- `CpuDevice`, `CpuBuffer`, and `CpuKernel` implement the executable reference backend.
- `SwarmEngine` owns or selects a device and maps `StandardOp` values to Symph operation labels/source and dispatch calls.
- `SwarmError` carries typed failure information, including unsupported backends, invalid sources, invalid arguments, and execution errors.

## Dispatch flow

1. The caller creates a `SwarmEngine` for a backend.
2. The device creates initialized or empty buffers from byte views (`silo::Arr<const uint8_t>`).
3. A standard or backend-specific kernel is compiled from a label and source description.
4. The caller supplies an array of `IComputeBuffer*` and a `WorkgroupDim` to dispatch.
5. The backend executes the kernel and reports a `SwarmError` status.
6. The caller reads a copied byte buffer and interprets it according to the operation's data contract.

The CPU backend maps standard operations to Symph's element-wise functions. Parallel execution can use `heist::Atelier`, while the buffer API remains backend independent.

## Backend strategy

The interfaces establish the stable contract for future devices. The Rust GPU and Cuda/Oxide implementations currently provide identity objects and return `UnsupportedBackend` from operations that are not implemented. This allows backend selection and API integration to be tested before native device runtimes exist.

## Memory and ownership

Devices return `std::unique_ptr` instances for buffers and kernels, making lifetime explicit. `Write` consumes a non-owning byte view; `Read` returns an independent `silo::Buff<uint8_t>` copy. Callers therefore cannot accidentally retain a view into a device's mutable storage.

## Dependencies and consumers

`swarm` depends on `silo` for byte views and owned readback, `symph` for standard operations, and optionally `heist` for parallel CPU execution. It is intended to sit between application code and backend-specific compute implementations.

## Invariants

- Buffer and kernel objects report the backend that created them.
- Dispatch receives buffers in the order required by the selected operation.
- Readback is an owned copy and is stable after subsequent device writes.
- Unsupported backends fail through typed errors rather than silently claiming success.
