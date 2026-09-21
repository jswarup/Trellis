use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// src/karst/_test/mod.rs
use	crate::karst::config::*;
use	crate::karst::fabric::KarstFabric;
use	crate::karst::fabric_node::KarstFabricNode;
use	crate::karst::host_node::HostResponse;
use	crate::karst::link::KarstFlit;
use crate::karst::MemoryFault;
use	crate::karst::noc::KarstNoc;
use crate::silo::useg::USeg;
use	crate::swarm::cpu::ComputeDevice;
use	crate::swarm::traits::WorkgroupDim;

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, ChannelStripesDoNotAlias, |ctx| {
    let mut fabric = KarstFabric::new();
    USeg::FromLen( 4).Traverse( |bank| {
        USeg::FromLen( 8).Traverse( |channel| {
            let addr = bank * 0x2000 + channel * 0x400;
            fabric.PostHostWrite( channel, addr, 0xAB00 + bank * 8 + channel);
        });
    });
    fabric.Advance( 100);
    USeg::FromLen( 4).Traverse( |bank| {
        USeg::FromLen( 8).Traverse( |channel| {
            let value = fabric.MemChan( channel).read_word( bank * 0x400).unwrap();
            jeeves_assert_eq!( ctx, value, 0xAB00 + bank * 8 + channel);
        });
    });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, SerialCycleTraceMatchesAcrossWorkerBudgets, |ctx| {
    let  	mut single = KarstFabric::with_workers( 1);
    let  	mut budgeted = KarstFabric::with_workers( 3);
    single.EnableCycleTrace( true);
    budgeted.EnableCycleTrace( true);
    single.PostHostWrite( 0, 0, 0xAABB_CCDD);
    budgeted.PostHostWrite( 0, 0, 0xAABB_CCDD);
    single.PostHostRead( 0, 0);
    budgeted.PostHostRead( 0, 0);
    single.Advance( 32);
    budgeted.Advance( 32);
    let  	single_trace = single.CycleTrace();
    let  	budgeted_trace = budgeted.CycleTrace();
    jeeves_assert_eq!( ctx, single_trace.Len(), 32);
    jeeves_assert_eq!( ctx, budgeted_trace.Len(), 32);
    jeeves_assert!( ctx, !single.CycleTraceOverflowed());
    jeeves_assert!( ctx, !budgeted.CycleTraceOverflowed());
    USeg::FromLen( single_trace.Len()).Traverse( |cycle| {
        jeeves_assert_eq!( ctx, single_trace[cycle], budgeted_trace[cycle]);
    });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, CycleTraceIsBoundedAndResettable, |ctx| {
    let  	mut fabric = KarstFabric::new();
    fabric.EnableCycleTrace( true);
    fabric.Advance( crate::karst::K_CYCLE_TRACE_CAPACITY + 1);
    jeeves_assert_eq!( ctx, fabric.CycleTrace().Len(), crate::karst::K_CYCLE_TRACE_CAPACITY);
    jeeves_assert!( ctx, fabric.CycleTraceOverflowed());
    fabric.ClearCycleTrace();
    jeeves_assert_eq!( ctx, fabric.CycleTrace().Len(), 0);
    jeeves_assert!( ctx, !fabric.CycleTraceOverflowed());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, CheckedHostRequestsRejectInvalidAddresses, |ctx| {
    let mut fabric = KarstFabric::new();
    let stats = fabric.Host( 0).stats();
    let unaligned = fabric.try_post_host_write( 0, 2, 0xCAFE_BABE);
    let outOfBounds = fabric.try_post_host_read( 0, 0x8000);
    let addressWidth = fabric.try_post_host_read( 0, 0x0100_0000);
    jeeves_assert_eq!( ctx, unaligned, Err( "Unaligned word access"));
    jeeves_assert_eq!( ctx, outOfBounds, Err( "Out of bounds memory access"));
    jeeves_assert_eq!( ctx, addressWidth, Err( "Address exceeds 24-bit flit width"));
    jeeves_assert_eq!( ctx, fabric.Host( 0).stats(), stats);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, InternallyInjectedMemoryFaultReturnsExplicitResponse, |ctx| {
    let device = ComputeDevice::WithWorkers( 1);
    let mut node = KarstFabricNode::new( &device, 0);
    let mut requestsAccepted = 0u32;
    let mut responses = 0u32;
    USeg::FromLen( 120).Traverse( |_| {
        let mut rxValid = [false; K_KL_PORTS_PER_HIND];
        let mut rxData = [0u64; K_KL_PORTS_PER_HIND];
        let txReady = [true; K_KL_PORTS_PER_HIND];
        if requestsAccepted < 2 {
            rxValid[0] = true;
            rxData[0] = KarstFlit::Pack( 0x8000, 0xCAFE_BABE, 0, requestsAccepted == 1);
        }
        if node.noc().kl_tx_valid( 0) {
            let response = KarstFlit::Unpack( node.noc().kl_tx_data( 0));
            jeeves_assert_eq!( ctx, response.ResponseFault(), Some( MemoryFault::OutOfBounds));
            let hostResponse = HostResponse::FromFlit( response);
            jeeves_assert_eq!( ctx, hostResponse.Fault(), Some( MemoryFault::OutOfBounds));
            jeeves_assert_eq!( ctx, hostResponse.Data(), Err( MemoryFault::OutOfBounds));
            responses += 1;
        }
        let accepted = rxValid[0] && node.noc().kl_rx_ready( 0);
        node.step( &rxValid, &rxData, &txReady);
        if accepted {
            requestsAccepted += 1;
        }
    });
    jeeves_assert_eq!( ctx, requestsAccepted, 2);
    jeeves_assert_eq!( ctx, responses, 2);
    jeeves_assert_eq!( ctx, node.mem_chan( 0).stats()._ReadsServiced, 0);
    jeeves_assert_eq!( ctx, node.mem_chan( 0).stats()._WritesServiced, 0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, TopologyWiring, |ctx| {
    let  	fabric = KarstFabric::new();
    // Verify 8 host nodes exist and are indexed 0..7
    for h in 0..K_HOSTS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.Host( h).host_id(), h);
    }
    // Verify 2 KarstHind Memory Fabric dies exist
    jeeves_assert_eq!( ctx, fabric.FabricNode( 0).die_id(), 0);
    jeeves_assert_eq!( ctx, fabric.FabricNode( 1).die_id(), 1);
    // Verify 8 physical DDR5 memory channels exist
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.MemChan( c).chan_idx(), c);
        jeeves_assert_eq!( ctx, fabric.MemChan( c).capacity(), 4096);
    }
    // Verify 8 VPUs exist
    for e in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.VPU( e).vpu_idx(), e);
        jeeves_assert_eq!( ctx, fabric.VPU( e).dispatches(), 0);
    }
    // Verify simulation engine initialized at cycle 0
    jeeves_assert_eq!( ctx, fabric.Engine()._CycleCount, 0);
    jeeves_println!( ctx, "         [Karst Topology Diagnostics]");
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

jeeves_test!( Karst, NocBackpressuresFullMemoryControllerQueue, |ctx| {
    let  	mut noc = KarstNoc::new( 0);
    let  	mut kl_rx_valid = [false; 10];
    let  	mut kl_rx_data = [0u64; 10];
    let  	kl_tx_ready = [true; 10];
    let  	mc_req_ready = [false; 4];                                 // Stall MC0
    let  	mc_resp_valid = [false; 4];
    let  	mc_resp_data = [0u64; 4];
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
        kl_rx_data[0] = KarstFlit::Pack( 0, request, 0, true);
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
    let  	ready_after_fill = noc.kl_rx_ready( 0);
    jeeves_assert_eq!( ctx, ready_after_fill, false);
    jeeves_println!( ctx, "         [NoC Backpressure Diagnostics]");
    jeeves_println!( ctx, "           Port Under Test     : KL0 (Ingress Port 0)");
    jeeves_println!( ctx, "           Target MC Stalled   : MC0 (McReqReady=0)");
    jeeves_println!( ctx, "           Flits Injected      : 32 requests");
    jeeves_println!( 
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

jeeves_test!( Karst, NocPipeBackpressureTransfersEachReadOnce, |ctx| {
    const REQUESTS: u32 = 60;
    let  	device = ComputeDevice::WithWorkers( 1);
    let  	mut node = KarstFabricNode::new( &device, 0);
    let  	mut posted = 0u32;
    let  	mut received = 0u32;
    for cycle in 0..1200u32 {
        let  	drain_responses = cycle >= 300;
        let  	mut kl_rx_valid = [false; K_KL_PORTS_PER_HIND];
        let  	mut kl_rx_data = [0u64; K_KL_PORTS_PER_HIND];
        let  	mut kl_tx_ready = [false; K_KL_PORTS_PER_HIND];
        kl_tx_ready[0] = drain_responses;
        if posted < REQUESTS {
            kl_rx_valid[0] = true;
            kl_rx_data[0] = KarstFlit::Pack( posted * 4, 0, 0, false);
        }
        if node.noc().kl_tx_valid( 0) && kl_tx_ready[0] {
            let  	response = KarstFlit::Unpack( node.noc().kl_tx_data( 0));
            jeeves_assert_eq!( ctx, response._Addr, received * 4);
            received += 1;
        }
        let  	accepted = kl_rx_valid[0] && node.noc().kl_rx_ready( 0);
        node.step( &kl_rx_valid, &kl_rx_data, &kl_tx_ready);
        if accepted {
            posted += 1;
        }
    }
    jeeves_assert_eq!( ctx, posted, REQUESTS);
    jeeves_assert_eq!( ctx, received, REQUESTS);
    jeeves_assert_eq!( ctx, node.mem_chan( 0).stats()._ReadsServiced, REQUESTS);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, SingleHostSingleDChanWrite, |ctx| {
    let  	mut fabric = KarstFabric::new();
    // Host 0 issues write to address 0x000 (Die 0, MC 0, DChan/MemChan 0)
    let  	test_addr = 0x000;
    let  	test_data = 0xCAFE_BABE;
    fabric.PostHostWrite( 0, test_addr, test_data);
    // Advance simulation through KarstLink, NoC, mpipe, and MC
    fabric.Advance( 25);
    // Verify memory channel 0 serviced write and holds correct value
    jeeves_assert_eq!( ctx, fabric.MemChan( 0).stats()._WritesServiced, 1);
    jeeves_assert_eq!( 
        ctx,
        fabric.MemChan( 0).read_word( test_addr).unwrap(),
        test_data
    );
    // Verify Host 0 stats recorded outgoing transaction
    jeeves_assert_eq!( ctx, fabric.Host( 0).stats()._WritesPosted, 1);
    jeeves_println!( 
        ctx,
        "         [Single Host Single MemChan Write Diagnostics]"
    );
    jeeves_println!( ctx, "           Host Origin         : Fore_0");
    jeeves_println!( ctx, "           Target Address      : 0x{:X}", test_addr);
    jeeves_println!( ctx, "           Write Data          : 0x{:X}", test_data);
    jeeves_println!( 
        ctx,
        "           Target Destination  : Die 0, MC 0 -> MemChan 0"
    );
    jeeves_println!( ctx, "           Simulation Advance  : 25 cycles elapsed");
    jeeves_println!( 
        ctx,
        "           MemChan 0 Verified  : 0x{:X} (Writes Serviced: {})",
        fabric.MemChan( 0).read_word( test_addr).unwrap(),
        fabric.MemChan( 0).stats()._WritesServiced
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, AllHostsRoundRobinInterleave, |ctx| {
    let  	mut fabric = KarstFabric::new();
    // Inject 8 write transactions (1 per host) with 1 kB address striping
    // Addresses 0x000, 0x400, 0x800, 0xC00 map to MemChan 0..3 (Die 0)
    // Addresses 0x1000, 0x1400, 0x1800, 0x1C00 map to MemChan 4..7 (Die 1)
    for h in 0..K_HOSTS_PER_FABRIC {
        let  	addr = h * 0x400;
        let  	data = 0x1000 + h;
        fabric.PostHostWrite( h, addr, data);
    }
    // Advance simulation to allow all 8 transactions to traverse fabric and settle
    fabric.Advance( 40);
    // Verify every DDR5 channel received exactly its interleaved portion
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.MemChan( c).stats()._WritesServiced, 1);
        let  	expected_data = 0x1000 + c;
        let  	local_addr = 0;
        jeeves_assert_eq!( 
            ctx,
            fabric.MemChan( c).read_word( local_addr).unwrap(),
            expected_data
        );
    }
    let  	st = fabric.Stats();
    jeeves_assert_eq!( ctx, st._TotalTxCount, 8);
    jeeves_assert_eq!( ctx, st._TotalBytesWritten, 32);
    jeeves_println!( 
        ctx,
        "         [Round-Robin 1 kB Address Striping Diagnostics]"
    );
    jeeves_println!( 
        ctx,
        "           Transactions Posted : 8 writes across 8 hosts"
    );
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        let  	a = c * 0x400;
        let  	d = 0x1000 + c;
        jeeves_println!( 
            ctx,
            "             Host {} -> Addr 0x{:X} -> MemChan {} (Data: 0x{:X}, Serviced: {})",
            c,
            a,
            c,
            d,
            fabric.MemChan( c).stats()._WritesServiced
        );
    }
    jeeves_println!( ctx, "           Simulation Advance  : 40 cycles elapsed");
    jeeves_println!( 
        ctx,
        "           Aggregate Metrics   : Total TX={}, Bytes Written={} B",
        st._TotalTxCount,
        st._TotalBytesWritten
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, VPUDispatch, |ctx| {
    let  	mut fabric = KarstFabric::new();
    // Initialize MemChan 0 with known pattern
    fabric.MemChanMut( 0).fill( 100);
    let  	verified_pre = fabric.MemChan( 0).verify( 100);
    jeeves_assert!( ctx, verified_pre);
    // Dispatch VPU 0 over MemChan 0 buffer (DoubleKernel operation)
    let  	err = fabric.dispatch_vpu( 0, 0, WorkgroupDim::Linear( 16));
    jeeves_assert!( ctx, err.is_ok());
    jeeves_assert_eq!( ctx, fabric.VPU( 0).dispatches(), 1);
    // Verify MemChan 0 buffer has been updated by VPU execution
    let  	verified_post = fabric.MemChan( 0).verify( 100);
    jeeves_assert!( ctx, !verified_post);
    jeeves_println!( ctx, "         [Near-Memory VPU Dispatch Diagnostics]");
    jeeves_println!( 
        ctx,
        "           Target Memory       : MemChan 0 (Physical DDR5 channel)"
    );
    jeeves_println!( 
        ctx,
        "           Initial Buffer Fill : Pattern 100 ({})",
        if verified_pre { "Verified" } else { "Failed" }
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
        fabric.VPU( 0).dispatches()
    );
    jeeves_println!( 
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

jeeves_test!( Karst, DualHindInterDieLink, |ctx| {
    let  	mut fabric = KarstFabric::new();
    // Host 0 writes to address 0x1000 (Die 1, MC 0, MemChan 4)
    let  	remote_addr = 0x1000;
    let  	remote_data = 0xDEAD_BEEF;
    fabric.PostHostWrite( 0, remote_addr, remote_data);
    fabric.Advance( 30);
    // Verify MemChan 4 on remote Die 1 received write
    jeeves_assert_eq!( ctx, fabric.MemChan( 4).stats()._WritesServiced, 1);
    jeeves_assert_eq!( 
        ctx,
        fabric
            .MemChan( 4)
            .read_word( remote_addr % ( fabric.MemChan( 4).capacity() as u32))
            .unwrap(),
        remote_data
    );
    // Host 0 issues read to the same remote address
    fabric.PostHostRead( 0, remote_addr);
    fabric.Advance( 30);
    // Verify Host 0 received read response
    let  	has_resp = fabric.Host( 0).has_responses();
    jeeves_assert!( ctx, has_resp);
    let  	mut resp = HostResponse::default();
    let  	popped = fabric.PopHostResponse( 0, &mut resp);
    jeeves_assert!( ctx, popped);
    jeeves_assert_eq!( ctx, resp._Addr, remote_addr);
    jeeves_assert_eq!( ctx, resp._Data, remote_data);
    jeeves_println!( ctx, "         [Dual-Hind Inter-Die Routing Diagnostics]");
    jeeves_println!( ctx, "           Source Host         : Fore_0 (Die 0 home)");
    jeeves_println!( 
        ctx,
        "           Target Address      : 0x{:X} (Die 1, MC 0, MemChan 4)",
        remote_addr
    );
    jeeves_println!( ctx, "           Remote Write Data   : 0x{:X}", remote_data);
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
    jeeves_println!( ctx, "           Total Round-Trip    : 60 cycles elapsed");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, ConfiguredWorkersPreserveTransport, |ctx| {
    // Worker configuration is retained for VPU work; fabric stepping is serial today.
    let  	mut fabric = KarstFabric::with_workers( 3);
    jeeves_assert_eq!( ctx, fabric.workers(), 3);
    for h in 0..K_HOSTS_PER_FABRIC {
        let  	addr = h * 0x400;
        let  	data = 0x2000 + h;
        fabric.PostHostWrite( h, addr, data);
    }
    fabric.Advance( 40);
    // Verify all 8 channels receive correct data with the configured worker budget.
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.MemChan( c).stats()._WritesServiced, 1);
        let  	expected_data = 0x2000 + c;
        let  	local_addr = 0;
        jeeves_assert_eq!( 
            ctx,
            fabric.MemChan( c).read_word( local_addr).unwrap(),
            expected_data
        );
    }
    jeeves_println!( ctx, "         [Configured Worker Transport Diagnostics]");
    jeeves_println!( 
        ctx,
        "           Worker Budget       : {} (VPU dispatch)",
        fabric.workers()
    );
    jeeves_println!( 
        ctx,
        "           Fabric Cycle Step   : Serial (no Heist work posted)"
    );
    jeeves_println!( ctx, "           Simulation Advance  : 40 cycles elapsed");
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        let  	a = c * 0x400;
        let  	local_addr = 0;
        jeeves_println!( 
            ctx,
            "             MemChan {} : Addr 0x{:X} = 0x{:X} (Serviced: {})",
            c,
            a,
            fabric.MemChan( c).read_word( local_addr).unwrap(),
            fabric.MemChan( c).stats()._WritesServiced
        );
    }
    jeeves_println!( 
        ctx,
        "           Transport Result    : All 8 channels verified bitwise identical"
    );
});

//-------------------------------------------------------------------------------------------------
// Console test
jeeves_test!( Karst, Console, Console, |ctx| {
    jeeves_println!( ctx, "         [Karst Console Test: Memory Fabric Active]");
});

//-------------------------------------------------------------------------------------------------
// Example test
jeeves_test!( Karst, Example, Example, |ctx| {
    let  	mut fabric = KarstFabric::new();
    fabric.PostHostWrite( 0, 0x100, 42);
    fabric.Advance( 25);
    jeeves_assert_eq!( ctx, fabric.MemChan( 0).read_word( 0x100).unwrap(), 42);
});

//-------------------------------------------------------------------------------------------------
// MemChan Bounds and Alignment Test
jeeves_test!( Karst, MemChanOOB, |ctx| {
    let  	mut fabric = KarstFabric::new();
    let  	chan = fabric.MemChanMut( 0);
    let  	cap = chan.capacity() as u32;
    // Unaligned write
    jeeves_assert!( ctx, chan.write_word( 0x1, 0xAA).is_err());
    // Out of bounds write
    jeeves_assert!( ctx, chan.write_word( cap - 2, 0xBB).is_err());
    jeeves_assert!( ctx, chan.write_word( cap, 0xCC).is_err());
    // Unaligned read
    jeeves_assert!( ctx, chan.read_word( 0x3).is_err());
    // Out of bounds read
    jeeves_assert!( ctx, chan.read_word( cap + 4).is_err());
});

//-------------------------------------------------------------------------------------------------
// Host Queue Saturation Test
jeeves_test!( Karst, HostQueueSaturation, |ctx| {
    let  	mut fabric = KarstFabric::new();
    let  	host = fabric.HostMut( 0);
    // Queue capacity is 32. Fill it exactly.
    for i in 0..32 {
        jeeves_assert!( ctx, host.post_write( i * 4, i).is_ok());
    }
    // 33rd transaction should fail with an error
    let  	overflow_res = host.post_write( 128, 42);
    jeeves_assert!( ctx, overflow_res.is_err());
});

//-------------------------------------------------------------------------------------------------
// Cycle Count Size Test
jeeves_test!( Karst, CycleCount, |ctx| {
    let  	mut fabric = KarstFabric::new();
    let  	ticks = fabric.Advance( 5);
    let  	engine = fabric.Engine();
    jeeves_assert_eq!( ctx, ticks, 5);
    jeeves_assert_eq!( ctx, engine._CycleCount, 5);
});

//-------------------------------------------------------------------------------------------------
// Lossless Handshake Under Saturation
jeeves_test!( Karst, LosslessHandshakeSaturation, |ctx| {
    let  	mut fabric = KarstFabric::new();
    // Host 0 posts 24 writes (exceeds individual FIFO capacity of 16)
    for i in 0..24 {
        fabric.PostHostWrite( 0, i * 4, 0x1000 + i);
    }
    // Advance simulation until all writes are serviced through saturated links
    fabric.Advance( 100);
    let  	stats = fabric.MemChan( 0).stats();
    jeeves_println!( ctx, "         [Lossless Handshake Diagnostics]");
    jeeves_println!( ctx, "           Writes Serviced: {}", stats._WritesServiced);
    jeeves_println!( ctx, "           Bytes Written:   {}", stats._BytesWritten);
    jeeves_assert_eq!( ctx, stats._WritesServiced, 24);
    jeeves_assert_eq!( ctx, stats._BytesWritten, 24 * 4);
    // Verify all written values in memory
    for i in 0..24 {
        let  	val = fabric.MemChan( 0).read_word( i * 4).unwrap();
        jeeves_assert_eq!( ctx, val, 0x1000 + i);
    }
    let  	host_stats = fabric.Host( 0).stats();
    jeeves_assert_eq!( ctx, host_stats._WritesPosted, 24);
    jeeves_assert_eq!( ctx, host_stats._TxCount, 24);
});

//-------------------------------------------------------------------------------------------------
// Lossless Handshake Under Read Response Queue Saturation
jeeves_test!( Karst, LosslessHandshakeReadSaturation, |ctx| {
    let  	mut fabric = KarstFabric::new();
    // Host 0 posts 24 writes
    for i in 0..24 {
        fabric.PostHostWrite( 0, i * 4, 0x5000 + i);
    }
    fabric.Advance( 60);
    // Host 0 posts 24 reads (exceeds MC response FIFO capacity of 16)
    for i in 0..24 {
        fabric.PostHostRead( 0, i * 4);
    }
    fabric.Advance( 100);
    // Verify all 24 responses arrived at Host 0 in order
    let  	mut resp = HostResponse::default();
    for i in 0..24 {
        let  	got = fabric.PopHostResponse( 0, &mut resp);
        jeeves_assert!( ctx, got);
        jeeves_assert_eq!( ctx, resp._Addr, i * 4);
        jeeves_assert_eq!( ctx, resp._Data, 0x5000 + i);
    }
    // No extra/duplicate responses
    jeeves_assert!( ctx, !fabric.PopHostResponse( 0, &mut resp));
    let  	stats = fabric.MemChan( 0).stats();
    jeeves_assert_eq!( ctx, stats._ReadsServiced, 24);
});

//-------------------------------------------------------------------------------------------------
// KarstFlit Little-Endian Byte Order Serialization Roundtrip
jeeves_test!( Karst, KarstFlitLittleEndianRoundtrip, |ctx| {
    let  	flit = KarstFlit::new( 0x0012_3456, 0xAABB_CCDD, 0x42, true);
    let  	bytes = flit.ToLeBytes();
    let  	unpacked = KarstFlit::FromLeBytes( bytes);
    jeeves_assert_eq!( ctx, unpacked._Addr, flit._Addr);
    jeeves_assert_eq!( ctx, unpacked._Data, flit._Data);
    jeeves_assert_eq!( ctx, unpacked._SrcId, flit._SrcId);
    jeeves_assert_eq!( ctx, unpacked._IsWrite, flit._IsWrite);
    // Verify raw u64 little-endian representation
    let  	raw = flit.to_raw();
    jeeves_assert_eq!( ctx, bytes, raw.to_le_bytes());
});

//-------------------------------------------------------------------------------------------------
// Karst MemChan Little-Endian Memory Wire Layout
jeeves_test!( Karst, KarstMemChanLittleEndianLayout, |ctx| {
    let  	mut fabric = KarstFabric::new();
    // Write 0x12345678 to address 0
    fabric.PostHostWrite( 0, 0, 0x1234_5678);
    fabric.Advance( 30);
    // Read directly from the underlying compute buffer to verify byte layout
    let  	chan = fabric.MemChan( 0);
    let  	mut raw_bytes = [0u8; 4];
    jeeves_assert!( ctx, chan.buffer().ReadAt( 0, ( &mut raw_bytes).into()).is_ok());
    // In Little-Endian: least significant byte first [0x78, 0x56, 0x34, 0x12]
    jeeves_assert_eq!( ctx, raw_bytes, [0x78, 0x56, 0x34, 0x12]);
});
