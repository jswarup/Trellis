# Trellis Agent Entry Point

Read and follow [agents/AGENTS.md](agents/AGENTS.md) for the engineering directives and [agents/FORMATTING.md](agents/FORMATTING.md) for Rust source layout.

The project aims to make human and AI collaboration produce simple, comprehensible code. Keep ownership, data location, mutation, traversal, and synchronization visible through recurring Trellis types and traits. Use `CpuOutputPartition` as the reference pattern for an API that makes a writable range and its handoff explicit.

Do not commit or push unless the user explicitly requests it.
