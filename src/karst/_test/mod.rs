// src/karst/_test/mod.rs

use crate::karst::config::*;
use crate::karst::fabric::KarstFabric;
use crate::karst::host_node::HostResponse;
use crate::karst::link::KarstFlit;
use crate::karst::noc::KarstNoc;
use crate::swarm::traits::WorkgroupDim;
use crate::{
    segue_assert, segue_assert_eq, segue_console_test, segue_example_test, segue_println,
    segue_test,
};

//-------------------------------------------------------------------------------------------------

segue_test!(Karst, TopologyWiring, |ctx| {
    let fabric = KarstFabric::new();

    // Verify 8 host nodes exist and are indexed 0..7
    for h in 0..K_HOSTS_PER_FABRIC {
        segue_assert_eq!(ctx, fabric.Host(h).host_id(), h);
    }

    // Verify 2 KarstHind Memory Fabric dies exist
    segue_assert_eq!(ctx, fabric.FabricNode(0).die_id(), 0);
    segue_assert_eq!(ctx, fabric.FabricNode(1).die_id(), 1);

    // Verify 8 physical DDR5 memory channels exist
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        segue_assert_eq!(ctx, fabric.MemChan(c).chan_idx(), c);
        segue_assert_eq!(ctx, fabric.MemChan(c).capacity(), 4096);
    }

    // Verify 8 VPUs exist
    for e in 0..K_MEM_CHANS_PER_FABRIC {
        segue_assert_eq!(ctx, fabric.VPU(e).vpu_idx(), e);
        segue_assert_eq!(ctx, fabric.VPU(e).dispatches(), 0);
    }

    // Verify simulation engine initialized at cycle 0
    segue_assert_eq!(ctx, fabric.Engine()._CycleCount, 0);

    segue_println!(ctx, "         [Karst Topology Diagnostics]");
    segue_println!(
        ctx,
        "           Hosts (Fore Dies)   : {} nodes (Fore_0..Fore_7)",
        K_HOSTS_PER_FABRIC
    );
    segue_println!(
        ctx,
        "           Fabric (Hind Dies)  : {} dies (Hind_0, Hind_1)",
        K_HIND_DIES_PER_FABRIC
    );
    segue_println!(
        ctx,
        "           DDR5 Memory Channels: {} channels (4 per Hind die, 4096 bytes each)",
        K_MEM_CHANS_PER_FABRIC
    );
    segue_println!(
        ctx,
        "           Near-Memory VPUs    : {} units (4 per Hind die)",
        K_MEM_CHANS_PER_FABRIC
    );
    segue_println!(
        ctx,
        "           Simulation Engine   : Initialized at cycle {}",
        fabric.Engine()._CycleCount
    );
});

//-------------------------------------------------------------------------------------------------

segue_test!(Karst, NocBackpressuresFullMemoryControllerQueue, |ctx| {
    let mut noc = KarstNoc::new(0);

    let mut kl_rx_valid = [false; 10];
    let mut kl_rx_data = [0u64; 10];
    let kl_tx_ready = [true; 10];
    let mc_req_ready = [false; 4]; // Stall MC0
    let mc_resp_valid = [false; 4];
    let mc_resp_data = [0u64; 4];

    noc.step(
        &kl_rx_valid,
        &kl_rx_data,
        &kl_tx_ready,
        &mc_req_ready,
        &mc_resp_valid,
        &mc_resp_data,
    );

    // Hold the MC stalled while filling its bounded request queue (32 requests)
    for request in 0..32 {
        kl_rx_data[0] = KarstFlit::Pack(0, request, 0, true);
        kl_rx_valid[0] = true;
        noc.step(
            &kl_rx_valid,
            &kl_rx_data,
            &kl_tx_ready,
            &mc_req_ready,
            &mc_resp_valid,
            &mc_resp_data,
        );
    }

    kl_rx_valid[0] = false;
    noc.step(
        &kl_rx_valid,
        &kl_rx_data,
        &kl_tx_ready,
        &mc_req_ready,
        &mc_resp_valid,
        &mc_resp_data,
    );

    let ready_after_fill = noc.kl_rx_ready(0);
    segue_assert_eq!(ctx, ready_after_fill, false);

    segue_println!(ctx, "         [NoC Backpressure Diagnostics]");
    segue_println!(ctx, "           Port Under Test     : KL0 (Ingress Port 0)");
    segue_println!(ctx, "           Target MC Stalled   : MC0 (McReqReady=0)");
    segue_println!(ctx, "           Flits Injected      : 32 requests");
    segue_println!(
        ctx,
        "           KlRxReady Asserted  : {}",
        if ready_after_fill {
            "true (Accepting)"
        } else {
            "false (Backpressure Active)"
        }
    );
});

//-------------------------------------------------------------------------------------------------

segue_test!(Karst, SingleHostSingleDChanWrite, |ctx| {
    let mut fabric = KarstFabric::new();

    // Host 0 issues write to address 0x000 (Die 0, MC 0, DChan/MemChan 0)
    let test_addr = 0x000;
    let test_data = 0xCAFE_BABE;

    fabric.PostHostWrite(0, test_addr, test_data);

    // Advance simulation through KarstLink, NoC, mpipe, and MC
    fabric.Advance(25);

    // Verify memory channel 0 serviced write and holds correct value
    segue_assert_eq!(ctx, fabric.MemChan(0).stats()._WritesServiced, 1);
    segue_assert_eq!(ctx, fabric.MemChan(0).read_word(test_addr), test_data);

    // Verify Host 0 stats recorded outgoing transaction
    segue_assert_eq!(ctx, fabric.Host(0).stats()._WritesPosted, 1);

    segue_println!(
        ctx,
        "         [Single Host Single MemChan Write Diagnostics]"
    );
    segue_println!(ctx, "           Host Origin         : Fore_0");
    segue_println!(ctx, "           Target Address      : 0x{:X}", test_addr);
    segue_println!(ctx, "           Write Data          : 0x{:X}", test_data);
    segue_println!(
        ctx,
        "           Target Destination  : Die 0, MC 0 -> MemChan 0"
    );
    segue_println!(ctx, "           Simulation Advance  : 25 cycles elapsed");
    segue_println!(
        ctx,
        "           MemChan 0 Verified  : 0x{:X} (Writes Serviced: {})",
        fabric.MemChan(0).read_word(test_addr),
        fabric.MemChan(0).stats()._WritesServiced
    );
});

//-------------------------------------------------------------------------------------------------

segue_test!(Karst, AllHostsRoundRobinInterleave, |ctx| {
    let mut fabric = KarstFabric::new();

    // Inject 8 write transactions (1 per host) with 1 kB address striping
    // Addresses 0x000, 0x400, 0x800, 0xC00 map to MemChan 0..3 (Die 0)
    // Addresses 0x1000, 0x1400, 0x1800, 0x1C00 map to MemChan 4..7 (Die 1)
    for h in 0..K_HOSTS_PER_FABRIC {
        let addr = h * 0x400;
        let data = 0x1000 + h;
        fabric.PostHostWrite(h, addr, data);
    }

    // Advance simulation to allow all 8 transactions to traverse fabric and settle
    fabric.Advance(40);

    // Verify every DDR5 channel received exactly its interleaved portion
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        segue_assert_eq!(ctx, fabric.MemChan(c).stats()._WritesServiced, 1);
        let expected_addr = c * 0x400;
        let expected_data = 0x1000 + c;
        segue_assert_eq!(
            ctx,
            fabric.MemChan(c).read_word(expected_addr),
            expected_data
        );
    }

    let st = fabric.Stats();
    segue_assert_eq!(ctx, st._TotalTxCount, 8);
    segue_assert_eq!(ctx, st._TotalBytesWritten, 32);

    segue_println!(
        ctx,
        "         [Round-Robin 1 kB Address Striping Diagnostics]"
    );
    segue_println!(
        ctx,
        "           Transactions Posted : 8 writes across 8 hosts"
    );
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        let a = c * 0x400;
        let d = 0x1000 + c;
        segue_println!(
            ctx,
            "             Host {} -> Addr 0x{:X} -> MemChan {} (Data: 0x{:X}, Serviced: {})",
            c,
            a,
            c,
            d,
            fabric.MemChan(c).stats()._WritesServiced
        );
    }
    segue_println!(ctx, "           Simulation Advance  : 40 cycles elapsed");
    segue_println!(
        ctx,
        "           Aggregate Metrics   : Total TX={}, Bytes Written={} B",
        st._TotalTxCount,
        st._TotalBytesWritten
    );
});

//-------------------------------------------------------------------------------------------------

segue_test!(Karst, VPUDispatch, |ctx| {
    let mut fabric = KarstFabric::new();

    // Initialize MemChan 0 with known pattern
    fabric.MemChanMut(0).fill(100);
    let verified_pre = fabric.MemChan(0).verify(100);
    segue_assert!(ctx, verified_pre);

    // Dispatch VPU 0 over MemChan 0 buffer (DoubleKernel operation)
    let err = fabric.dispatch_vpu(0, 0, WorkgroupDim::Linear(16));
    segue_assert!(ctx, err.is_ok());
    segue_assert_eq!(ctx, fabric.VPU(0).dispatches(), 1);

    // Verify MemChan 0 buffer has been updated by VPU execution
    let verified_post = fabric.MemChan(0).verify(100);
    segue_assert!(ctx, !verified_post);

    segue_println!(ctx, "         [Near-Memory VPU Dispatch Diagnostics]");
    segue_println!(
        ctx,
        "           Target Memory       : MemChan 0 (Physical DDR5 channel)"
    );
    segue_println!(
        ctx,
        "           Initial Buffer Fill : Pattern 100 ({})",
        if verified_pre { "Verified" } else { "Failed" }
    );
    segue_println!(
        ctx,
        "           Kernel Dispatched   : DoubleKernel (Swarm SIMT)"
    );
    segue_println!(
        ctx,
        "           Workgroup Dimension : Linear(16) (16 threadblocks)"
    );
    segue_println!(
        ctx,
        "           Dispatch Status     : {}",
        if err.is_ok() { "Success (Ok)" } else { "Error" }
    );
    segue_println!(
        ctx,
        "           Dispatches Recorded : {}",
        fabric.VPU(0).dispatches()
    );
    segue_println!(
        ctx,
        "           Memory Mutated      : {}",
        if !verified_post {
            "Buffer values updated in-place"
        } else {
            "No mutation observed"
        }
    );
});

//-------------------------------------------------------------------------------------------------

segue_test!(Karst, DualHindInterDieLink, |ctx| {
    let mut fabric = KarstFabric::new();

    // Host 0 writes to address 0x1000 (Die 1, MC 0, MemChan 4)
    let remote_addr = 0x1000;
    let remote_data = 0xDEAD_BEEF;

    fabric.PostHostWrite(0, remote_addr, remote_data);
    fabric.Advance(30);

    // Verify MemChan 4 on remote Die 1 received write
    segue_assert_eq!(ctx, fabric.MemChan(4).stats()._WritesServiced, 1);
    segue_assert_eq!(ctx, fabric.MemChan(4).read_word(remote_addr), remote_data);

    // Host 0 issues read to the same remote address
    fabric.PostHostRead(0, remote_addr);
    fabric.Advance(30);

    // Verify Host 0 received read response
    let has_resp = fabric.Host(0).has_responses();
    segue_assert!(ctx, has_resp);
    let mut resp = HostResponse::default();
    let popped = fabric.PopHostResponse(0, &mut resp);
    segue_assert!(ctx, popped);
    segue_assert_eq!(ctx, resp._Addr, remote_addr);
    segue_assert_eq!(ctx, resp._Data, remote_data);

    segue_println!(ctx, "         [Dual-Hind Inter-Die Routing Diagnostics]");
    segue_println!(ctx, "           Source Host         : Fore_0 (Die 0 home)");
    segue_println!(
        ctx,
        "           Target Address      : 0x{:X} (Die 1, MC 0, MemChan 4)",
        remote_addr
    );
    segue_println!(ctx, "           Remote Write Data   : 0x{:X}", remote_data);
    segue_println!(
        ctx,
        "           Write Traversal     : Fore_0 -> KL0 -> Hind_0 -> Inter-Die KL8 -> Hind_1 -> MemChan 4 (30 cycles)"
    );
    segue_println!(
        ctx,
        "           Read Traversal      : Fore_0 -> Hind_0 -> Inter-Die KL8 -> Hind_1 -> MemChan 4 (30 cycles)"
    );
    segue_println!(
        ctx,
        "           Response Verified   : Addr=0x{:X}, Data=0x{:X}",
        resp._Addr,
        resp._Data
    );
    segue_println!(ctx, "           Total Round-Trip    : 60 cycles elapsed");
});

//-------------------------------------------------------------------------------------------------

segue_test!(Karst, ParallelDrive, |ctx| {
    // Execute fabric in parallel mode with 4 workers
    let mut fabric = KarstFabric::with_workers(4);

    for h in 0..K_HOSTS_PER_FABRIC {
        let addr = h * 0x400;
        let data = 0x2000 + h;
        fabric.PostHostWrite(h, addr, data);
    }

    fabric.Advance(40);

    // Verify all 8 channels received correct data under parallel execution
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        segue_assert_eq!(ctx, fabric.MemChan(c).stats()._WritesServiced, 1);
        let expected_addr = c * 0x400;
        let expected_data = 0x2000 + c;
        segue_assert_eq!(
            ctx,
            fabric.MemChan(c).read_word(expected_addr),
            expected_data
        );
    }

    segue_println!(ctx, "         [Parallel SimEngine Drive Diagnostics]");
    segue_println!(
        ctx,
        "           Execution Mode      : Parallel (4 Heist worker threads)"
    );
    segue_println!(
        ctx,
        "           Concurrent Hosts    : 8 hosts issuing writes simultaneously"
    );
    segue_println!(ctx, "           Simulation Advance  : 40 cycles elapsed");
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        let a = c * 0x400;
        segue_println!(
            ctx,
            "             MemChan {} : Addr 0x{:X} = 0x{:X} (Serviced: {})",
            c,
            a,
            fabric.MemChan(c).read_word(a),
            fabric.MemChan(c).stats()._WritesServiced
        );
    }
    segue_println!(
        ctx,
        "           Parallel Parity     : All 8 channels verified bitwise identical"
    );
});

//-------------------------------------------------------------------------------------------------
// Console test

segue_console_test!(Karst, Console, |ctx| {
    segue_println!(ctx, "         [Karst Console Test: Memory Fabric Active]");
});

//-------------------------------------------------------------------------------------------------
// Example test

segue_example_test!(Karst, Example, |ctx| {
    let mut fabric = KarstFabric::new();
    fabric.PostHostWrite(0, 0x100, 42);
    fabric.Advance(25);
    segue_assert_eq!(ctx, fabric.MemChan(0).read_word(0x100), 42);
});
