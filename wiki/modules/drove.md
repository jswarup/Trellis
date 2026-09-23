# Drove: Rust-GPU SPIR-V Shaders & Host Program Contracts

**Path:** `src/drove/`  
**Crate Member:** `trellis::drove` (Host Contracts) and nested `drove` (Rust-GPU Workspace Member)  
**Status:** GPU Shader Compilation Framework

---

## 1. Module Overview & Mission

`drove` represents Trellis's GPU shader architecture. It operates across two distinct build environments:
1. **The Nested Rust-GPU Crate (`src/drove/src/`)**: A separate `no_std` Cargo workspace crate using `spirv-std` that compiles native Rust code into Vulkan-compatible SPIR-V binaries via `rustc_codegen_spirv` in `tools/build.rs`.
2. **The Host Module (`src/drove/`)**: Compiles as part of `trellis`, exposing typed host entry points (`ComputeEntryPoint`, `GeometryEntryPoint`) and embedding the compiled SPIR-V bytecode directly into the binary.

### Design Principles
- **Rust-to-SPIR-V Native Compilation**: Authoring compute and graphics kernels in idiomatic Rust rather than GLSL or HLSL.
- **Embedded SPIR-V Bytecode**: The compiled `.spv` artifact is embedded at build time (`include_bytes!`), guaranteeing zero external asset dependency at runtime.
- **Strong Entry Point Typing**: Enums and descriptor types prevent mismatched shader stage or binding index errors on the host.

---

## 2. Component Hierarchy & File Map

```mermaid
flowchart TD
    subgraph HostContracts
        HostMod[mod.rs: Drove Host Module]
        ComputeHost[compute.rs: ComputeEntryPoint, ComputeSpirV]
        GeomHost[geometry.rs: GeometryEntryPoint]
        CompHost[composite.rs: CompositeEntryPoint]
    end
    subgraph BuildScript
        BuildRs[tools/build.rs: SpirvBuilder Invocation]
        SpvArtifact[DROVE_SPV_PATH / embedded bytes]
    end
    subgraph NestedShaderCrate
        ShaderLib[src/drove/src/lib.rs: #![no_std] spirv-std]
        ComputeKernel[src/drove/src/compute.rs: double_cs]
    end

    BuildRs -->|Compiles| ShaderLib
    ShaderLib --> ComputeKernel
    ComputeKernel --> SpvArtifact
    SpvArtifact -->|Embedded into| ComputeHost
    ComputeHost --> HostMod
```

| File | Primary Types & Functions | Description |
|---|---|---|
| `src/drove/mod.rs` | Re-exports | Host-side public entry points and descriptors. |
| `src/drove/compute.rs` | `ComputeEntryPoint`, `ComputeSpirV` | Embeds the compiled SPIR-V bytecode and defines compute entry point names (`double_cs`). |
| `src/drove/geometry.rs` | `GeometryEntryPoint`, `GeometryFragmentEntryPoint` | Host descriptors for vertex and fragment mesh rendering pipelines. |
| `src/drove/composite.rs` | `CompositeEntryPoint` | Host descriptors for post-process composition shaders. |
| `src/drove/Cargo.toml` | Package manifest | Configures `spirv-std = "0.9"` for `spirv-unknown-vulkan1.1`. |
| `src/drove/src/lib.rs` | Shader crate entry | `#![no_std]` Rust-GPU crate entry point. |
| `src/drove/src/compute.rs` | `double_cs` | SPIR-V compute kernel multiplying buffer elements by 2.0. |

---

## 3. Build & Compilation Topology

1. During `cargo build`, Cargo executes `tools/build.rs`.
2. `tools/build.rs` invokes `spirv_builder::SpirvBuilder::new("src/drove", "spirv-unknown-vulkan1.1")`.
3. The generated SPIR-V file is placed in `target/` and its path is exposed via the environment variable `DROVE_SPV_PATH`.
4. `src/drove/compute.rs` imports the binary via `include_bytes!(env!("DROVE_SPV_PATH"))`.

---

## 4. Integration Boundaries

- **Upstream Dependencies**: `spirv-std`, `spirv-builder` (in `build.rs`).
- **Downstream Consumers**:
  - `swarm::engine`: Selects `Drove` compute kernels when hardware backends are configured.
  - Future graphics paths: Designed to replace WGSL shaders in `swarm::viewport`.
