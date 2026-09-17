// src/karst/config.rs

//-------------------------------------------------------------------------------------------------

// Topology constants mirroring Karst(8, 8) balanced host-to-channel architecture.
pub const K_HOSTS_PER_FABRIC: u32 = 8;                                 // KarstFore IO dies (Hosts 0..7)
pub const K_MEM_CHANS_PER_FABRIC: u32 = 8;                             // Total DDR5 memory channels (0..7)
pub const K_HIND_DIES_PER_FABRIC: u32 = 2;                             // KarstHind Memory Fabric dies (0 and 1)
pub const K_MC_PER_HIND: u32 = 4;                                      // Memory controllers per KarstHind die
pub const K_VPU_PER_HIND: u32 = 4;                                     // Near-memory VPUs per KarstHind die
pub const K_LINK_DEPTH: usize = 4;                                     // Pipeline stages per retimed link (mpipe FIFO depth)
// Port and routing constants
pub const K_KL_PORTS_PER_HIND: usize = 10;                             // KarstLink ports per Hind die (0..9)
pub const K_MC_PORTS_PER_HIND: usize = 4;                              // Memory controller ports per Hind die (0..3)
pub const K_DIE_ADDR_BIT: u32 = 12;                                    // Bit 12 selects target Hind die (0 or 1)
pub const K_MC_ADDR_SHIFT: u32 = 10;                                   // Bits 11:10 select local memory controller (0..3)
pub const K_MC_ADDR_MASK: u32 = 0x3;                                   // 2-bit mask for 4 local MCs
pub const K_HOSTS_PER_HIND: usize = 4;                                 // Primary-homed hosts per Hind die
pub const K_INTERDIE_PORT_BASE: usize = 8;                             // KL ports 8..9 used for inter-die link
