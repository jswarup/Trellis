// config.h -------------------------------------------------------------------------------------------------------
#pragma once

#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// Topology constants mirroring Karst (X, Y) balanced host-to-channel architecture.

constexpr uint32_t      k_HostsPerFabric    = 8;                        // KarstFore IO dies (Karst X)
constexpr uint32_t      k_DChansPerFabric   = 8;                        // Total DDR5 memory channels (Karst Y)
constexpr uint32_t      k_HindDiesPerFabric   = 2;                        // KarstHind Memory Fabric dies
constexpr uint32_t      k_McPerHind           = 4;                        // Memory controllers per KarstHind die
constexpr uint32_t      k_KarstPortsPerHind      = 10;                       // KarstLink ports per KarstHind die
constexpr uint32_t      k_KarstPortsPerFore      = 2;                        // KarstLink ports per KarstFore die
constexpr uint32_t      k_VPUPerHind          = 4;                        // Near-memory EPUs per KarstHind die
constexpr uint32_t      k_LinkDepth         = 4;                        // Pipeline stages per retimed link

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

