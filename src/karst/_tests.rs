use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
// src/karst/_test/mod.rs
// src/karst/_tests.rs
use crate::karst::config::*;
use crate::karst::fabric::KarstFabric;
use crate::karst::host_node::HostResponse;
use crate::karst::link::KarstFlit;
use crate::karst::noc::KarstNoc;
use crate::silo::USeg;
use crate::swarm::traits::WorkgroupDim;

//-------------------------------------------------------------------------------------------------

jeeves_test!(Karst, TopologyWiring, |ctx| {
    let fabric = KarstFabric::new();
    let fabric = KarstFabric::New();
    // Verify 8 host nodes exist and are indexed 0..7
    for h in 0..K_HOSTS_PER_FABRIC {
        jeeves_assert_eq!(ctx, fabric.Host(h).host_id(), h);
    }
    USeg::FromLen(K_HOSTS_PER_FABRIC).Traverse(|h| {
        jeeves_assert_eq!(ctx, fabric.Host(h).HostId(), h);
    });
    // Verify 2 KarstHind Memory Fabric dies exist
    jeeves_assert_eq!(ctx, fabric.FabricNode(0).die_id(), 0);
    jeeves_assert_eq!(ctx, fabric.FabricNode(1).die_id(), 1);
    jeeves_assert_eq!(ctx, fabric.FabricNode(0).DieId(), 0);
    jeeves_assert_eq!(ctx, fabric.FabricNode(1).DieId(), 1);
    // Verify 8 physical DDR5 memory channels exist
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!(ctx, fabric.MemChan(c).chan_idx(), c);
        jeeves_assert_eq!(ctx, fabric.MemChan(c).capacity(), 4096);
    }
    USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|c| {
        jeeves_assert_eq!(ctx, fabric.DChan(c).ChanIdx(), c);
        jeeves_assert_eq!(ctx, fabric.DChan(c).Capacity(), 4096);
    });
    // Verify 8 VPUs exist
    for e in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!(ctx, fabric.VPU(e).vpu_idx(), e);
        jeeves_assert_eq!(ctx, fabric.VPU(e).dispatches(), 0);
    }
    USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|e| {
        jeeves_assert_eq!(ctx, fabric.VPU(e).VPUIdx(), e);
        jeeves_assert_eq!(ctx, fabric.VPU(e).Dispatches(), 0);
    });
    // Verify simulation engine initialized at cycle 0
    jeeves_assert_eq!(ctx, fabric.Engine()._CycleCount, 0);
    jeeves_println!(ctx, "         [Karst Topology Diagnostics]");
    jeeves_println!(
        ctx,
        "           Hosts (Fore Dies)   : {} nodes (Fore_0..Fore_7)",
        K_HOSTS_PER_FABRIC
    );
    jeeves_println!(
        ctx,
        "           Fabric (Hind Dies)  : {} dies (Hind_0, Hind_1)",
        K_HIND_DIES_PER_FABRIC
    );
    jeeves_println!(
        ctx,
        "           DDR5 Memory Channels: {} channels (4 per Hind die, 4096 bytes each)",
        K_MEM_CHANS_PER_FABRIC
    );
    jeeves_println!(
        ctx,
        "           Near-Memory VPUs    : {} units (4 per Hind die)",
        K_MEM_CHANS_PER_FABRIC
    );
    jeeves_println!(
        ctx,
        "           Simulation Engine   : Initialized at cycle {}",
        fabric.Engine()._CycleCount
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Karst, NocBackpressuresFullMemoryControllerQueue, |ctx| {
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
    let mut noc = KarstNoc::New(0);
    let mut klRxValid = [false; 10];
    let mut klRxData = [0u64; 10];
    let klTxReady = [true; 10];
    let mcReqReady = [false; 4]; // Stall MC0
    let mcRespValid = [false; 4];
    let mcRespData = [0u64; 4];
    noc.Step(
        &klRxValid,
        &klRxData,
        &klTxReady,
        &mcReqReady,
        &mcRespValid,
        &mcRespData,
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
    USeg::FromLen(32).Traverse(|request| {
        klRxData[0] = KarstFlit::Pack(0, request, 0, true);
        klRxValid[0] = true;
        noc.Step(
            &klRxValid,
            &klRxData,
            &klTxReady,
            &mcReqReady,
            &mcRespValid,
            &mcRespData,
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
    });
    klRxValid[0] = false;
    noc.Step(
        &klRxValid,
        &klRxData,
        &klTxReady,
        &mcReqReady,
        &mcRespValid,
        &mcRespData,
    );
    let ready_after_fill = noc.kl_rx_ready(0);
    jeeves_assert_eq!(ctx, ready_after_fill, false);
    let readyAfterFill = noc.KlRxReady(0);
    jeeves_assert_eq!(ctx, readyAfterFill, false);
    jeeves_println!(ctx, "         [NoC Backpressure Diagnostics]");
    jeeves_println!(ctx, "           Port Under Test     : KL0 (Ingress Port 0)");
    jeeves_println!(ctx, "           Target MC Stalled   : MC0 (McReqReady=0)");
    jeeves_println!(ctx, "           Flits Injected      : 32 requests");
    jeeves_println!(
        ctx,
        "           KlRxReady Asserted  : {}",
        if ready_after_fill {
        if readyAfterFill {
            "true (Accepting)"
        } else {
            "false (Backpressure Active)"
        }
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Karst, SingleHostSingleDChanWrite, |ctx| {
    let mut fabric = KarstFabric::new();
    let mut fabric = KarstFabric::New();
    // Host 0 issues write to address 0x000 (Die 0, MC 0, DChan/MemChan 0)
    let test_addr = 0x000;
    let test_data = 0xCAFE_BABE;
    fabric.PostHostWrite(0, test_addr, test_data);
    let testAddr = 0x000;
    let testData = 0xCAFE_BABE;
    fabric.PostHostWrite(0, testAddr, testData);
    // Advance simulation through KarstLink, NoC, mpipe, and MC
    fabric.Advance(25);
    // Verify memory channel 0 serviced write and holds correct value
    jeeves_assert_eq!(ctx, fabric.MemChan(0).stats()._WritesServiced, 1);
    jeeves_assert_eq!(ctx, fabric.DChan(0).Stats()._WritesServiced, 1);
    jeeves_assert_eq!(
        ctx,
        fabric.MemChan(0).read_word(test_addr).unwrap(),
        test_data
        fabric.DChan(0).ReadWord(testAddr).unwrap(),
        testData
    );
    // Verify Host 0 stats recorded outgoing transaction
    jeeves_assert_eq!(ctx, fabric.Host(0).stats()._WritesPosted, 1);
    jeeves_assert_eq!(ctx, fabric.Host(0).Stats()._WritesPosted, 1);
    jeeves_println!(
        ctx,
        "         [Single Host Single MemChan Write Diagnostics]"
    );
    jeeves_println!(ctx, "           Host Origin         : Fore_0");
    jeeves_println!(ctx, "           Target Address      : 0x{:X}", test_addr);
    jeeves_println!(ctx, "           Write Data          : 0x{:X}", test_data);
    jeeves_println!(ctx, "           Target Address      : 0x{:X}", testAddr);
    jeeves_println!(ctx, "           Write Data          : 0x{:X}", testData);
    jeeves_println!(
        ctx,
        "           Target Destination  : Die 0, MC 0 -> MemChan 0"
    );
    jeeves_println!(ctx, "           Simulation Advance  : 25 cycles elapsed");
    jeeves_println!(
        ctx,
        "           MemChan 0 Verified  : 0x{:X} (Writes Serviced: {})",
        fabric.MemChan(0).read_word(test_addr).unwrap(),
        fabric.MemChan(0).stats()._WritesServiced
        fabric.DChan(0).ReadWord(testAddr).unwrap(),
        fabric.DChan(0).Stats()._WritesServiced
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Karst, AllHostsRoundRobinInterleave, |ctx| {
    let mut fabric = KarstFabric::new();
    let mut fabric = KarstFabric::New();
    // Inject 8 write transactions (1 per host) with 1 kB address striping
    // Addresses 0x000, 0x400, 0x800, 0xC00 map to MemChan 0..3 (Die 0)
    // Addresses 0x1000, 0x1400, 0x1800, 0x1C00 map to MemChan 4..7 (Die 1)
    for h in 0..K_HOSTS_PER_FABRIC {
    USeg::FromLen(K_HOSTS_PER_FABRIC).Traverse(|h| {
        let addr = h * 0x400;
        let data = 0x1000 + h;
        fabric.PostHostWrite(h, addr, data);
    }
    });
    // Advance simulation to allow all 8 transactions to traverse fabric and settle
    fabric.Advance(40);
    // Verify every DDR5 channel received exactly its interleaved portion
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!(ctx, fabric.MemChan(c).stats()._WritesServiced, 1);
        let expected_addr = c * 0x400;
        let expected_data = 0x1000 + c;
        let local_addr = expected_addr % (fabric.MemChan(c).capacity() as u32);
    USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|c| {
        jeeves_assert_eq!(ctx, fabric.DChan(c).Stats()._WritesServiced, 1);
        let expectedAddr = c * 0x400;
        let expectedData = 0x1000 + c;
        let localAddr = expectedAddr % (fabric.DChan(c).Capacity() as u32);
        jeeves_assert_eq!(
            ctx,
            fabric.MemChan(c).read_word(local_addr).unwrap(),
            expected_data
            fabric.DChan(c).ReadWord(localAddr).unwrap(),
            expectedData
        );
    }
    });
    let st = fabric.Stats();
    jeeves_assert_eq!(ctx, st._TotalTxCount, 8);
    jeeves_assert_eq!(ctx, st._TotalBytesWritten, 32);
    jeeves_println!(
        ctx,
        "         [Round-Robin 1 kB Address Striping Diagnostics]"
    );
    jeeves_println!(
        ctx,
        "           Transactions Posted : 8 writes across 8 hosts"
    );
    for c in 0..K_MEM_CHANS_PER_FABRIC {
    USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|c| {
        let a = c * 0x400;
        let d = 0x1000 + c;
        jeeves_println!(
            ctx,
            "             Host {} -> Addr 0x{:X} -> MemChan {} (Data: 0x{:X}, Serviced: {})",
            c,
            a,
            c,
            d,
            fabric.MemChan(c).stats()._WritesServiced
            fabric.DChan(c).Stats()._WritesServiced
        );
    }
    });
    jeeves_println!(ctx, "           Simulation Advance  : 40 cycles elapsed");
    jeeves_println!(
        ctx,
        "           Aggregate Metrics   : Total TX={}, Bytes Written={} B",
        st._TotalTxCount,
        st._TotalBytesWritten
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Karst, VPUDispatch, |ctx| {
    let mut fabric = KarstFabric::new();
    let mut fabric = KarstFabric::New();
    // Initialize MemChan 0 with known pattern
    fabric.MemChanMut(0).fill(100);
    let verified_pre = fabric.MemChan(0).verify(100);
    jeeves_assert!(ctx, verified_pre);
    fabric.DChanMut(0).Fill(100);
    let verifiedPre = fabric.DChan(0).Verify(100);
    jeeves_assert!(ctx, verifiedPre);
    // Dispatch VPU 0 over MemChan 0 buffer (DoubleKernel operation)
    let err = fabric.dispatch_vpu(0, 0, WorkgroupDim::Linear(16));
    let err = fabric.DispatchVPU(0, 0, WorkgroupDim::Linear(16));
    jeeves_assert!(ctx, err.is_ok());
    jeeves_assert_eq!(ctx, fabric.VPU(0).dispatches(), 1);
    jeeves_assert_eq!(ctx, fabric.VPU(0).Dispatches(), 1);
    // Verify MemChan 0 buffer has been updated by VPU execution
    let verified_post = fabric.MemChan(0).verify(100);
    jeeves_assert!(ctx, !verified_post);
    let verifiedPost = fabric.DChan(0).Verify(100);
    jeeves_assert!(ctx, !verifiedPost);
    jeeves_println!(ctx, "         [Near-Memory VPU Dispatch Diagnostics]");
    jeeves_println!(
        ctx,
        "           Target Memory       : MemChan 0 (Physical DDR5 channel)"
    );
    jeeves_println!(
        ctx,
        "           Initial Buffer Fill : Pattern 100 ({})",
        if verified_pre { "Verified" } else { "Failed" }
        if verifiedPre { "Verified" } else { "Failed" }
    );
    jeeves_println!(
        ctx,
        "           Kernel Dispatched   : DoubleKernel (Swarm SIMT)"
    );
    jeeves_println!(
        ctx,
        "           Workgroup Dimension : Linear(16) (16 threadblocks)"
    );
    jeeves_println!(
        ctx,
        "           Dispatch Status     : {}",
        if err.is_ok() { "Success (Ok)" } else { "Error" }
    );
    jeeves_println!(
        ctx,
        "           Dispatches Recorded : {}",
        fabric.VPU(0).dispatches()
        fabric.VPU(0).Dispatches()
    );
    jeeves_println!(
        ctx,
        "           Memory Mutated      : {}",
        if !verified_post {
        if !verifiedPost {
            "Buffer values updated in-place"
        } else {
            "No mutation observed"
        }
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Karst, DualHindInterDieLink, |ctx| {
    let mut fabric = KarstFabric::new();
    let mut fabric = KarstFabric::New();
    // Host 0 writes to address 0x1000 (Die 1, MC 0, MemChan 4)
    let remote_addr = 0x1000;
    let remote_data = 0xDEAD_BEEF;
    fabric.PostHostWrite(0, remote_addr, remote_data);
    let remoteAddr = 0x1000;
    let remoteData = 0xDEAD_BEEF;
    fabric.PostHostWrite(0, remoteAddr, remoteData);
    fabric.Advance(30);
    // Verify MemChan 4 on remote Die 1 received write
    jeeves_assert_eq!(ctx, fabric.MemChan(4).stats()._WritesServiced, 1);
    jeeves_assert_eq!(ctx, fabric.DChan(4).Stats()._WritesServiced, 1);
    jeeves_assert_eq!(
        ctx,
        fabric
            .MemChan(4)
            .read_word(remote_addr % (fabric.MemChan(4).capacity() as u32))
            .DChan(4)
            .ReadWord(remoteAddr % (fabric.DChan(4).Capacity() as u32))
            .unwrap(),
        remote_data
        remoteData
    );
    // Host 0 issues read to the same remote address
    fabric.PostHostRead(0, remote_addr);
    fabric.PostHostRead(0, remoteAddr);
    fabric.Advance(30);
    // Verify Host 0 received read response
    let has_resp = fabric.Host(0).has_responses();
    jeeves_assert!(ctx, has_resp);
    let hasResp = fabric.Host(0).HasResponses();
    jeeves_assert!(ctx, hasResp);
    let mut resp = HostResponse::default();
    let popped = fabric.PopHostResponse(0, &mut resp);
    jeeves_assert!(ctx, popped);
    jeeves_assert_eq!(ctx, resp._Addr, remote_addr);
    jeeves_assert_eq!(ctx, resp._Data, remote_data);
    jeeves_assert_eq!(ctx, resp._Addr, remoteAddr);
    jeeves_assert_eq!(ctx, resp._Data, remoteData);
    jeeves_println!(ctx, "         [Dual-Hind Inter-Die Routing Diagnostics]");
    jeeves_println!(ctx, "           Source Host         : Fore_0 (Die 0 home)");
    jeeves_println!(
        ctx,
        "           Target Address      : 0x{:X} (Die 1, MC 0, MemChan 4)",
        remote_addr
        remoteAddr
    );
    jeeves_println!(ctx, "           Remote Write Data   : 0x{:X}", remote_data);
    jeeves_println!(ctx, "           Remote Write Data   : 0x{:X}", remoteData);
    jeeves_println!(
        ctx,
        "           Write Traversal     : Fore_0 -> KL0 -> Hind_0 -> Inter-Die KL8 -> Hind_1 -> MemChan 4 (30 cycles)"
    );
    jeeves_println!(
        ctx,
        "           Read Traversal      : Fore_0 -> Hind_0 -> Inter-Die KL8 -> Hind_1 -> MemChan 4 (30 cycles)"
    );
    jeeves_println!(
        ctx,
        "           Response Verified   : Addr=0x{:X}, Data=0x{:X}",
        resp._Addr,
        resp._Data
    );
    jeeves_println!(ctx, "           Total Round-Trip    : 60 cycles elapsed");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Karst, ParallelDrive, |ctx| {
    // Execute fabric in parallel mode with 4 workers
    let mut fabric = KarstFabric::with_workers(4);
    for h in 0..K_HOSTS_PER_FABRIC {
    let mut fabric = KarstFabric::WithWorkers(4);
    USeg::FromLen(K_HOSTS_PER_FABRIC).Traverse(|h| {
        let addr = h * 0x400;
        let data = 0x2000 + h;
        fabric.PostHostWrite(h, addr, data);
    }
    });
    fabric.Advance(40);
    // Verify all 8 channels received correct data under parallel execution
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!(ctx, fabric.MemChan(c).stats()._WritesServiced, 1);
        let expected_addr = c * 0x400;
        let expected_data = 0x2000 + c;
        let local_addr = expected_addr % (fabric.MemChan(c).capacity() as u32);
    USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|c| {
        jeeves_assert_eq!(ctx, fabric.DChan(c).Stats()._WritesServiced, 1);
        let expectedAddr = c * 0x400;
        let expectedData = 0x2000 + c;
        let localAddr = expectedAddr % (fabric.DChan(c).Capacity() as u32);
        jeeves_assert_eq!(
            ctx,
            fabric.MemChan(c).read_word(local_addr).unwrap(),
            expected_data
            fabric.DChan(c).ReadWord(localAddr).unwrap(),
            expectedData
        );
    }
    });
    jeeves_println!(ctx, "         [Parallel SimEngine Drive Diagnostics]");
    jeeves_println!(
        ctx,
        "           Execution Mode      : Parallel (4 Heist worker threads)"
    );
    jeeves_println!(
        ctx,
        "           Concurrent Hosts    : 8 hosts issuing writes simultaneously"
    );
    jeeves_println!(ctx, "           Simulation Advance  : 40 cycles elapsed");
    for c in 0..K_MEM_CHANS_PER_FABRIC {
    USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|c| {
        let a = c * 0x400;
        let local_addr = a % (fabric.MemChan(c).capacity() as u32);
        let localAddr = a % (fabric.DChan(c).Capacity() as u32);
        jeeves_println!(
            ctx,
            "             MemChan {} : Addr 0x{:X} = 0x{:X} (Serviced: {})",
            c,
            a,
            fabric.MemChan(c).read_word(local_addr).unwrap(),
            fabric.MemChan(c).stats()._WritesServiced
            fabric.DChan(c).ReadWord(localAddr).unwrap(),
            fabric.DChan(c).Stats()._WritesServiced
        );
    }
    });
    jeeves_println!(
        ctx,
        "           Parallel Parity     : All 8 channels verified bitwise identical"
    );
});

//-------------------------------------------------------------------------------------------------

// Console test
jeeves_test!(Karst, Console, Console, |ctx| {
    jeeves_println!(ctx, "         [Karst Console Test: Memory Fabric Active]");
});

//-------------------------------------------------------------------------------------------------

// Example test
jeeves_test!(Karst, Example, Example, |ctx| {
    let mut fabric = KarstFabric::new();
    let mut fabric = KarstFabric::New();
    fabric.PostHostWrite(0, 0x100, 42);
    fabric.Advance(25);
    jeeves_assert_eq!(ctx, fabric.MemChan(0).read_word(0x100).unwrap(), 42);
    jeeves_assert_eq!(ctx, fabric.DChan(0).ReadWord(0x100).unwrap(), 42);
});

//-------------------------------------------------------------------------------------------------

// MemChan Bounds and Alignment Test
jeeves_test!(Karst, MemChanOOB, |ctx| {
    let mut fabric = KarstFabric::new();
    let chan = fabric.MemChanMut(0);
    let cap = chan.capacity() as u32;
    let mut fabric = KarstFabric::New();
    let chan = fabric.DChanMut(0);
    let cap = chan.Capacity() as u32;
    // Unaligned write
    jeeves_assert!(ctx, chan.write_word(0x1, 0xAA).is_err());
    jeeves_assert!(ctx, chan.WriteWord(0x1, 0xAA).is_err());
    // Out of bounds write
    jeeves_assert!(ctx, chan.write_word(cap - 2, 0xBB).is_err());
    jeeves_assert!(ctx, chan.write_word(cap, 0xCC).is_err());
    jeeves_assert!(ctx, chan.WriteWord(cap - 2, 0xBB).is_err());
    jeeves_assert!(ctx, chan.WriteWord(cap, 0xCC).is_err());
    // Unaligned read
    jeeves_assert!(ctx, chan.read_word(0x3).is_err());
    jeeves_assert!(ctx, chan.ReadWord(0x3).is_err());
    // Out of bounds read
    jeeves_assert!(ctx, chan.read_word(cap + 4).is_err());
    jeeves_assert!(ctx, chan.ReadWord(cap + 4).is_err());
});

//-------------------------------------------------------------------------------------------------

// Host Queue Saturation Test
jeeves_test!(Karst, HostQueueSaturation, |ctx| {
    let mut fabric = KarstFabric::new();
    let mut fabric = KarstFabric::New();
    let host = fabric.HostMut(0);
    // Queue capacity is 32. Fill it exactly.
    for i in 0..32 {
        jeeves_assert!(ctx, host.post_write(i * 4, i).is_ok());
    }
    USeg::FromLen(32).Traverse(|i| {
        jeeves_assert!(ctx, host.PostWrite(i * 4, i).is_ok());
    });
    // 33rd transaction should fail with an error
    let overflow_res = host.post_write(128, 42);
    jeeves_assert!(ctx, overflow_res.is_err());
    let overflowRes = host.PostWrite(128, 42);
    jeeves_assert!(ctx, overflowRes.is_err());
});

//-------------------------------------------------------------------------------------------------

// Cycle Count Size Test
jeeves_test!(Karst, CycleCount, |ctx| {
    let mut fabric = KarstFabric::new();
    let mut fabric = KarstFabric::New();
    let ticks = fabric.Advance(5);
    let engine = fabric.Engine();
    jeeves_assert_eq!(ctx, ticks, 5);
    jeeves_assert_eq!(ctx, engine._CycleCount, 5);
});
