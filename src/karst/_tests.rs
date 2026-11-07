// src/karst/_tests.rs
//
// ================================================================================================
// Karst(8, 8) Architecture & Memory Fabric Test Suite
// ================================================================================================
//
// Topology Overview:
//   - Balanced Karst(8, 8) Host-to-Channel Architecture:
//       * 8 KarstFore IO Die Hosts (Fore_0 .. Fore_7).
//       * 2 KarstHind Memory Fabric Dies (Hind_0 and Hind_1).
//       * 8 Physical DDR5 Memory Channels (MemChan 0..7, 4096 bytes capacity each):
//           - Hind_0: MemChan 0..3 (Local Memory Controllers MC0..MC3).
//           - Hind_1: MemChan 4..7 (Local Memory Controllers MC0..MC3).
//       * 8 Near-Memory Vector Processing Units (VPU 0..7):
//           - Hind_0: VPU 0..3 coupled to MemChan 0..3.
//           - Hind_1: VPU 4..7 coupled to MemChan 4..7.
//
//   - Interconnect & Routing Topology:
//       * Each Hind die exposes 10 KarstLink (KL) ports (KL0..KL9):
//           - KL0..KL3: Primary-homed Fore hosts (Fore_0..3 on Hind_0, Fore_4..7 on Hind_1).
//           - KL4..KL7: Cross-homed Fore hosts (Fore_4..7 on Hind_0, Fore_0..3 on Hind_1).
//           - KL8..KL9: Bidirectional Inter-Die Links (Hind_0 <-> Hind_1).
//       * Each host possesses dual KarstLink interfaces (Link0 and Link1) with 32-entry FIFOs.
//       * Retimed memory pipelines (mpipe) insert 4 pipeline stages (K_LINK_DEPTH = 4) between NoC and MCs.
//       * Lossless credit/backpressure flow control across links, NoC crossbars, and MC FIFOs.
//
//   - Addressing & Interleaving:
//       * 24-bit physical flit address space:
//           - Bit 12 (K_DIE_ADDR_BIT): Target Hind Die selection (0 = Hind_0, 1 = Hind_1).
//           - Bits 11:10 (K_MC_ADDR_SHIFT): Local MC / MemChan selection (0..3), providing 1 kB striping.
//           - Bits 9:0: Byte offset within the 4096-byte MemChan.
//           - Bits 1:0: Must be 00 for aligned 4-byte word accesses.
// ================================================================================================

use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_test };
use	crate::fenst::Xplr;
use	crate::karst::config::*;
use	crate::karst::fabric::{ KarstFabric, VpuDispatchDesc };
use	crate::karst::fabric_node::KarstFabricNode;
use	crate::karst::host_node::HostResponse;
use	crate::karst::link::{ KarstFlit, KarstLink, KarstLinkChannel };
use	crate::karst::MemoryFault;
use	crate::karst::noc::KarstNoc;
use	crate::silo::useg::USeg;
use	crate::swarm::cpu::ComputeDevice;
use	crate::swarm::traits::{ SwarmErrorKind, WorkgroupDim };

//-------------------------------------------------------------------------------------------------


//-------------------------------------------------------------------------------------------------
// UI Example: Karst Fabric Explorer
//
// Topology:
//   Full Karst(8, 8) memory fabric: 8 KarstFore IO hosts (Fore_0..Fore_7), 2 KarstHind dies
//   (Hind_0, Hind_1), 8 DDR5 memory channels (MemChan 0..7), and 8 near-memory VPUs (VPU 0..7).
// Purpose:
//   Registers an interactive inspectable example into the Fascia UI Examples browser (ExampleBrowser),
//   allowing users to visually navigate and inspect the Karst fabric hierarchy, per-host metrics,
//   memory channel contents, and diagnostic counters.
// Implementation:
//   Instantiates KarstFabric, issues transactions across multiple hosts, advances the simulation,
//   and constructs an interactive KarstNodeXplr tree (Hosts, Memory Channels, Aggregate Diagnostics)
//   conforming to the XplrExample protocol.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, FabricExplorer, XplrExample, |_ctx| {
    let mut fabric = KarstFabric::new();
    fabric.post_host_write( 0, 0x000, 0xCAFE_BABE);
    fabric.post_host_write( 4, 0x1000, 0xDEAD_BEEF);
    fabric.step_cycle();

    Ok(Box::new(fabric.IntoCircuit()) as Box<dyn Xplr + Send>)
});

jeeves_test!( Karst, DieCircuit, XplrExample, |_ctx| {
    let mut fabric = KarstFabric::new();
    fabric.post_host_write( 0, 0x100, 42);
    fabric.step_cycle();

    let die_circuit = crate::karst::circuit_xplr::KarstDieCircuit::New(std::sync::Arc::new(fabric), 0);
    Ok(Box::new(die_circuit) as Box<dyn Xplr + Send>)
});

//-------------------------------------------------------------------------------------------------
// Test 1: FlitAndLinkWireFormat
//
// Topology:
//   KarstLink point-to-point channels, 64-bit NoC flit protocol layer (KarstFlit), and DDR5
//   Memory Channel 0 (MemChan 0) compute storage layout.
// Purpose:
//   Validates link/channel initial readiness, 64-bit flit little-endian serialization/deserialization
//   roundtrip fidelity, and little-endian word layout inside underlying memory compute buffers.
// Implementation:
//   Checks KarstLinkChannel and KarstLink default vs new constructors, packs/unpacks KarstFlit
//   with ToLeBytes/FromLeBytes verifying all fields and raw u64 little-endian bytes, and posts a
//   host write to MemChan 0 to confirm byte order [0x78, 0x56, 0x34, 0x12] via direct buffer read.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, FlitAndLinkWireFormat, |ctx| {
    // 1. KarstLink and KarstLinkChannel constructor defaults
    let  	channel = KarstLinkChannel::default();
    jeeves_assert_eq!( ctx, channel, KarstLinkChannel::new());
    jeeves_assert!( ctx, channel.ready());
    jeeves_assert!( ctx, !channel.is_transfer());

    let  	link = KarstLink::default();
    jeeves_assert_eq!( ctx, link, KarstLink::new());
    jeeves_assert!( ctx, link.tx().ready());
    jeeves_assert!( ctx, link.rx().ready());

    // 2. 64-bit KarstFlit serialization roundtrip
    let  	flit = KarstFlit::new( 0x0012_3456, 0xAABB_CCDD, 0x42, true);
    let  	bytes = flit.ToLeBytes();
    let  	unpacked = KarstFlit::FromLeBytes( bytes);
    jeeves_assert_eq!( ctx, unpacked._Addr, flit._Addr);
    jeeves_assert_eq!( ctx, unpacked._Data, flit._Data);
    jeeves_assert_eq!( ctx, unpacked._SrcId, flit._SrcId);
    jeeves_assert_eq!( ctx, unpacked._IsWrite, flit._IsWrite);
    jeeves_assert_eq!( ctx, bytes, flit.to_raw().to_le_bytes());

    // 3. Compute buffer little-endian memory layout
    let  	mut fabric = KarstFabric::new();
    fabric.PostHostWrite( 0, 0, 0x1234_5678);
    fabric.Advance( 30);
    let  	chan = fabric.MemChan( 0);
    let  	mut raw_bytes = [0u8; 4];
    jeeves_assert!( ctx, chan.buffer().ReadAt( 0, ( &mut raw_bytes).into()).is_ok());
    jeeves_assert_eq!( ctx, raw_bytes, [0x78, 0x56, 0x34, 0x12]);
});

//-------------------------------------------------------------------------------------------------
// Test 2: FabricTopologyAndEndToEndAccess
//
// Topology:
//   Complete Karst(8, 8) architecture: 8 KarstFore IO hosts, 2 KarstHind dies, 8 DDR5 memory
//   channels (4 per die, 4096 bytes each), 8 near-memory VPUs, and top-level simulation engine.
// Purpose:
//   Validates structural integrity and indexing of the Karst topology wiring, confirms cycle engine
//   advancement, and executes an end-to-end memory write traversal from Host 0 to MemChan 0.
// Implementation:
//   Validates host IDs 0..7, die IDs 0 and 1, memory channel capacities, VPU indices, and initial cycle 0.
//   Posts a write from Host 0 to address 0x000 with 0xCAFE_BABE, advances 25 cycles, verifies write
//   commit and host stats, then advances 5 cycles confirming cycle counter reaches 30.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, FabricTopologyAndEndToEndAccess, |ctx| {
    let  	mut fabric = KarstFabric::new();

    // Verify 8 hosts, 2 dies, 8 memory channels, and 8 VPUs
    for h in 0..K_HOSTS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.Host( h).host_id(), h);
    }
    jeeves_assert_eq!( ctx, fabric.FabricNode( 0).die_id(), 0);
    jeeves_assert_eq!( ctx, fabric.FabricNode( 1).die_id(), 1);
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.MemChan( c).chan_idx(), c);
        jeeves_assert_eq!( ctx, fabric.MemChan( c).capacity(), 4096);
        jeeves_assert_eq!( ctx, fabric.VPU( c).vpu_idx(), c);
        jeeves_assert_eq!( ctx, fabric.VPU( c).dispatches(), 0);
    }
    jeeves_assert_eq!( ctx, fabric.Engine()._CycleCount, 0);

    // End-to-end write from Host 0 to MemChan 0
    let  	test_addr = 0x000;
    let  	test_data = 0xCAFE_BABE;
    fabric.PostHostWrite( 0, test_addr, test_data);
    fabric.Advance( 25);
    jeeves_assert_eq!( ctx, fabric.MemChan( 0).stats()._WritesServiced, 1);
    jeeves_assert_eq!( ctx, fabric.MemChan( 0).read_word( test_addr).unwrap(), test_data);
    jeeves_assert_eq!( ctx, fabric.Host( 0).stats()._WritesPosted, 1);

    // Engine cycle count increment verification
    let  	ticks = fabric.Advance( 5);
    jeeves_assert_eq!( ctx, ticks, 30);
    jeeves_assert_eq!( ctx, fabric.Engine()._CycleCount, 30);
});

//-------------------------------------------------------------------------------------------------
// Test 3: FabricAddressInterleavingAndRouting
//
// Topology:
//   Complete Karst(8, 8) topology: 8 Fore hosts, 2 Hind dies, 8 DDR5 memory channels, and
//   bidirectional inter-die link KL8. Bit 12 selects Hind die; bits 11:10 select local MC.
// Purpose:
//   Verifies 1 kB address striping across all 8 channels concurrently, proves multi-bank memory
//   isolation without aliasing, and validates cross-die routing over the inter-die link (KL8).
// Implementation:
//   1. All 8 hosts write concurrently to h * 0x400; verifies each channel serviced its write.
//   2. 4 banks x 8 channels striped writes; verifies unaliased storage across banks and channels.
//   3. Host 0 writes and reads remote address 0x1000 on Die 1 via KL8, verifying response match.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, FabricAddressInterleavingAndRouting, |ctx| {
    let  	mut fabric = KarstFabric::new();

    // 1. All-hosts concurrent 1 kB address striping (Channels 0..7)
    for h in 0..K_HOSTS_PER_FABRIC {
        fabric.PostHostWrite( h, h * 0x400, 0x1000 + h);
    }
    fabric.Advance( 40);
    for c in 0..K_MEM_CHANS_PER_FABRIC {
        jeeves_assert_eq!( ctx, fabric.MemChan( c).stats()._WritesServiced, 1);
        jeeves_assert_eq!( ctx, fabric.MemChan( c).read_word( 0).unwrap(), 0x1000 + c);
    }
    let  	st = fabric.Stats();
    jeeves_assert_eq!( ctx, st._TotalTxCount, 8);
    jeeves_assert_eq!( ctx, st._TotalBytesWritten, 32);

    // 2. Multi-bank isolation without aliasing (4 banks x 8 channels)
    USeg::FromLen( 4).Traverse( |bank| {
        USeg::FromLen( 8).Traverse( |channel| {
            let  	addr = bank * 0x2000 + channel * 0x400;
            fabric.PostHostWrite( channel, addr, 0xAB00 + bank * 8 + channel);
        });
    });
    fabric.Advance( 100);
    USeg::FromLen( 4).Traverse( |bank| {
        USeg::FromLen( 8).Traverse( |channel| {
            let  	value = fabric.MemChan( channel).read_word( bank * 0x400).unwrap();
            jeeves_assert_eq!( ctx, value, 0xAB00 + bank * 8 + channel);
        });
    });

    // 3. Cross-die routing over inter-die link KL8 (Host 0 -> Hind_0 -> KL8 -> Hind_1 -> MemChan 4)
    let  	mut remote_fabric = KarstFabric::new();
    let  	remote_addr = 0x1000;
    let  	remote_data = 0xDEAD_BEEF;
    remote_fabric.PostHostWrite( 0, remote_addr, remote_data);
    remote_fabric.Advance( 30);
    jeeves_assert_eq!( ctx, remote_fabric.MemChan( 4).stats()._WritesServiced, 1);
    jeeves_assert_eq!(
        ctx,
        remote_fabric.MemChan( 4).read_word( remote_addr % ( remote_fabric.MemChan( 4).capacity() as u32)).unwrap(),
        remote_data
    );

    remote_fabric.PostHostRead( 0, remote_addr);
    remote_fabric.Advance( 30);
    let  	mut resp = HostResponse::default();
    jeeves_assert!( ctx, remote_fabric.PopHostResponse( 0, &mut resp));
    jeeves_assert_eq!( ctx, resp._Addr, remote_addr);
    jeeves_assert_eq!( ctx, resp._Data, remote_data);
});

//-------------------------------------------------------------------------------------------------
// Test 4: HostAndMemChanBoundaryValidation
//
// Topology:
//   Fore Host 0 ingress queue (capacity 32) and DDR5 physical memory channel (capacity 4096 bytes).
// Purpose:
//   Validates boundary enforcement and alignment checking across the host and memory interface,
//   verifying that misaligned offsets, out-of-bounds addresses, and queue saturation are rejected.
// Implementation:
//   Tests MemChan direct word access boundaries (aligned last word Ok, misaligned and OOB Err),
//   tested host checked APIs (unaligned, OOB, >24-bit address rejection), and fills host queue to
//   32 entries asserting the 33rd post returns Err.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, HostAndMemChanBoundaryValidation, |ctx| {
    let  	mut fabric = KarstFabric::new();

    // 1. MemChan direct read/write bounds and alignment
    let  	chan = fabric.MemChanMut( 0);
    let  	cap = chan.capacity() as u32;
    let  	last_word = cap - 4;
    jeeves_assert!( ctx, chan.write_word( last_word, 0xDEAD_BEEF).is_ok());
    jeeves_assert_eq!( ctx, chan.read_word( last_word), Ok( 0xDEAD_BEEF));
    jeeves_assert!( ctx, chan.write_word( 0x1, 0xAA).is_err());
    jeeves_assert!( ctx, chan.write_word( cap - 2, 0xBB).is_err());
    jeeves_assert!( ctx, chan.write_word( cap, 0xCC).is_err());
    jeeves_assert!( ctx, chan.read_word( 0x3).is_err());
    jeeves_assert!( ctx, chan.read_word( cap + 4).is_err());

    // 2. Checked host request API validation
    let  	stats = fabric.Host( 0).stats();
    let  	unaligned = fabric.try_post_host_write( 0, 2, 0xCAFE_BABE);
    let  	outOfBounds = fabric.try_post_host_read( 0, 0x8000);
    let  	addressWidth = fabric.try_post_host_read( 0, 0x0100_0000);
    jeeves_assert_eq!( ctx, unaligned, Err( "Unaligned word access"));
    jeeves_assert_eq!( ctx, outOfBounds, Err( "Out of bounds memory access"));
    jeeves_assert_eq!( ctx, addressWidth, Err( "Address exceeds 24-bit flit width"));
    jeeves_assert_eq!( ctx, fabric.Host( 0).stats(), stats);

    // 3. Host queue capacity saturation (capacity = 32)
    let  	host = fabric.HostMut( 0);
    for i in 0..32 {
        jeeves_assert!( ctx, host.post_write( i * 4, i).is_ok());
    }
    let  	overflow_res = host.post_write( 128, 42);
    jeeves_assert!( ctx, overflow_res.is_err());
});

//-------------------------------------------------------------------------------------------------
// Test 5: NocFaultInjectionAndDraining
//
// Topology:
//   Isolated KarstFabricNode (Hind Die 0) with KarstNoc ingress port KL0 and retimed egress queues.
// Purpose:
//   Verifies that out-of-bounds and unaligned flits injected directly into NoC ingress generate
//   explicit MemoryFault error responses without modifying memory, and verifies that saturated
//   fault queues drain completely without loss or duplication.
// Implementation:
//   Injects out-of-bounds flits and asserts MemoryFault::OutOfBounds response; injects unaligned flit
//   and asserts MemoryFault::Unaligned response. Stalls egress for 300 cycles while posting 60 fault
//   requests, then un-stalls and confirms all 60 responses drain exactly once.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, NocFaultInjectionAndDraining, |ctx| {
    let  	device = ComputeDevice::WithWorkers( 1);

    // 1. Direct out-of-bounds fault response verification
    let  	mut node = KarstFabricNode::new( &device, 0);
    let  	mut requestsAccepted = 0u32;
    let  	mut responses = 0u32;
    USeg::FromLen( 120).Traverse( |_| {
        let  	mut rxValid = [false; K_KL_PORTS_PER_HIND];
        let  	mut rxData = [0u64; K_KL_PORTS_PER_HIND];
        let  	txReady = [true; K_KL_PORTS_PER_HIND];
        if requestsAccepted < 2 {
            rxValid[0] = true;
            rxData[0] = KarstFlit::Pack( 0x8000, 0xCAFE_BABE, 0, requestsAccepted == 1);
        }
        if node.noc().kl_tx_valid( 0) {
            let  	response = KarstFlit::Unpack( node.noc().kl_tx_data( 0));
            jeeves_assert_eq!( ctx, response.ResponseFault(), Some( MemoryFault::OutOfBounds));
            let  	hostResponse = HostResponse::FromFlit( response);
            jeeves_assert_eq!( ctx, hostResponse.Fault(), Some( MemoryFault::OutOfBounds));
            jeeves_assert_eq!( ctx, hostResponse.Data(), Err( MemoryFault::OutOfBounds));
            responses += 1;
        }
        let  	accepted = rxValid[0] && node.noc().kl_rx_ready( 0);
        node.step( &rxValid, &rxData, &txReady);
        if accepted {
            requestsAccepted += 1;
        }
    });
    jeeves_assert_eq!( ctx, requestsAccepted, 2);
    jeeves_assert_eq!( ctx, responses, 2);
    jeeves_assert_eq!( ctx, node.mem_chan( 0).stats()._ReadsServiced, 0);
    jeeves_assert_eq!( ctx, node.mem_chan( 0).stats()._WritesServiced, 0);

    // 2. Direct unaligned fault response verification
    let  	mut node_unaligned = KarstFabricNode::new( &device, 0);
    let  	mut injected = false;
    let  	mut unaligned_responses = 0u32;
    USeg::FromLen( 48).Traverse( |_| {
        let  	mut rx_valid = [false; K_KL_PORTS_PER_HIND];
        let  	mut rx_data = [0u64; K_KL_PORTS_PER_HIND];
        let  	tx_ready = [true; K_KL_PORTS_PER_HIND];
        if !injected {
            rx_valid[0] = true;
            rx_data[0] = KarstFlit::Pack( 2, 0, 0, false);
        }
        if node_unaligned.noc().kl_tx_valid( 0) {
            let  	response = KarstFlit::Unpack( node_unaligned.noc().kl_tx_data( 0));
            jeeves_assert_eq!( ctx, response.ResponseFault(), Some( MemoryFault::Unaligned));
            unaligned_responses += 1;
        }
        let  	accepted = rx_valid[0] && node_unaligned.noc().kl_rx_ready( 0);
        node_unaligned.step( &rx_valid, &rx_data, &tx_ready);
        injected |= accepted;
    });
    jeeves_assert!( ctx, injected);
    jeeves_assert_eq!( ctx, unaligned_responses, 1);

    // 3. Saturated fault response queue draining (60 requests drained under backpressure)
    const REQUESTS: u32 = 60;
    let  	mut node_drain = KarstFabricNode::new( &device, 0);
    let  	mut posted = 0u32;
    let  	mut received = 0u32;
    USeg::FromLen( 1200).Traverse( |cycle| {
        let  	mut rx_valid = [false; K_KL_PORTS_PER_HIND];
        let  	mut rx_data = [0u64; K_KL_PORTS_PER_HIND];
        let  	mut tx_ready = [false; K_KL_PORTS_PER_HIND];
        tx_ready[0] = cycle >= 300;
        if posted < REQUESTS {
            rx_valid[0] = true;
            rx_data[0] = KarstFlit::Pack( 0x8000, 0, 0, false);
        }
        if node_drain.noc().kl_tx_valid( 0) && tx_ready[0] {
            let  	response = KarstFlit::Unpack( node_drain.noc().kl_tx_data( 0));
            jeeves_assert_eq!( ctx, response.ResponseFault(), Some( MemoryFault::OutOfBounds));
            received += 1;
        }
        let  	accepted = rx_valid[0] && node_drain.noc().kl_rx_ready( 0);
        node_drain.step( &rx_valid, &rx_data, &tx_ready);
        if accepted {
            posted += 1;
        }
    });
    jeeves_assert_eq!( ctx, posted, REQUESTS);
    jeeves_assert_eq!( ctx, received, REQUESTS);
});

//-------------------------------------------------------------------------------------------------
// Test 6: NocAndPipeBackpressure
//
// Topology:
//   KarstNoc router and 4-stage retimed memory pipeline (mpipe) on Hind Die 0.
// Purpose:
//   Verifies credit/backpressure flow control when a memory controller stalls, and proves that
//   backpressure across retimed NoC pipelines services and returns each read request exactly once.
// Implementation:
//   1. Stalls MC0 (mc_req_ready=0), injects 32 write requests to port KL0, confirms kl_rx_ready(0)
//      deasserts (false), activating backpressure.
//   2. Injects 60 read requests with stalled egress (cycle < 300), then drains responses, confirming
//      exactly 60 requests posted, 60 received in address order, and 60 reads serviced by MemChan 0.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, NocAndPipeBackpressure, |ctx| {
    // 1. NoC credit exhaustion backpressure on full MC queue
    let  	mut noc = KarstNoc::new( 0);
    let  	mut kl_rx_valid = [false; 10];
    let  	mut kl_rx_data = [0u64; 10];
    let  	kl_tx_ready = [true; 10];
    let  	mc_req_ready = [false; 4];
    let  	mc_resp_valid = [false; 4];
    let  	mc_resp_data = [0u64; 4];
    noc.step( &kl_rx_valid, &kl_rx_data, &kl_tx_ready, &mc_req_ready, &mc_resp_valid, &mc_resp_data);

    for request in 0..32 {
        kl_rx_data[0] = KarstFlit::Pack( 0, request, 0, true);
        kl_rx_valid[0] = true;
        noc.step( &kl_rx_valid, &kl_rx_data, &kl_tx_ready, &mc_req_ready, &mc_resp_valid, &mc_resp_data);
    }
    kl_rx_valid[0] = false;
    noc.step( &kl_rx_valid, &kl_rx_data, &kl_tx_ready, &mc_req_ready, &mc_resp_valid, &mc_resp_data);
    jeeves_assert_eq!( ctx, noc.kl_rx_ready( 0), false);

    // 2. Retimed pipeline backpressure services and returns each read exactly once
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
// Test 7: FabricLosslessSaturationAndStats
//
// Topology:
//   Fore Host 0 (Fore_0) connected to Hind Die 0 (Hind_0) over KarstLink 0 (KL0), routing into
//   Memory Controller 0 (capacity 16) and DDR5 Channel 0.
// Purpose:
//   Validates lossless backpressured handshake handling under write and read queue saturation
//   (24 transactions vs 16 FIFO capacity), while verifying link and queue high-water mark tracking.
// Implementation:
//   Posts 24 writes, advances 100 cycles, verifies all 24 writes serviced and values intact. Posts
//   24 reads, advances 100 cycles, pops all 24 responses in order. Confirms KarstLinkStats and
//   KarstQueueStats record accepted flits and high-water occupancy marks.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, FabricLosslessSaturationAndStats, |ctx| {
    let  	mut fabric = KarstFabric::new();

    // 1. Lossless write saturation (24 writes > FIFO capacity of 16)
    for i in 0..24 {
        fabric.PostHostWrite( 0, i * 4, 0x1000 + i);
    }
    fabric.Advance( 100);
    let  	chan_stats = fabric.MemChan( 0).stats();
    jeeves_assert_eq!( ctx, chan_stats._WritesServiced, 24);
    jeeves_assert_eq!( ctx, chan_stats._BytesWritten, 24 * 4);
    for i in 0..24 {
        jeeves_assert_eq!( ctx, fabric.MemChan( 0).read_word( i * 4).unwrap(), 0x1000 + i);
    }
    let  	host_stats = fabric.Host( 0).stats();
    jeeves_assert_eq!( ctx, host_stats._WritesPosted, 24);
    jeeves_assert_eq!( ctx, host_stats._TxCount, 24);

    // 2. Lossless read response saturation (24 reads > MC response FIFO capacity of 16)
    let  	mut read_fabric = KarstFabric::new();
    for i in 0..24 {
        read_fabric.PostHostWrite( 0, i * 4, 0x5000 + i);
    }
    read_fabric.Advance( 60);
    for i in 0..24 {
        read_fabric.PostHostRead( 0, i * 4);
    }
    read_fabric.Advance( 100);
    let  	mut resp = HostResponse::default();
    for i in 0..24 {
        let  	got = read_fabric.PopHostResponse( 0, &mut resp);
        jeeves_assert!( ctx, got);
        jeeves_assert_eq!( ctx, resp._Addr, i * 4);
        jeeves_assert_eq!( ctx, resp._Data, 0x5000 + i);
    }
    jeeves_assert!( ctx, !read_fabric.PopHostResponse( 0, &mut resp));
    jeeves_assert_eq!( ctx, read_fabric.MemChan( 0).stats()._ReadsServiced, 24);

    // 3. Link and queue statistics verification
    let  	link_stats = read_fabric.LinkStats();
    jeeves_assert!( ctx, link_stats._IngressAccepted[0][0] >= 2);
    jeeves_assert!( ctx, link_stats._EgressAccepted[0][0] >= 1);
    jeeves_assert_eq!( ctx, link_stats._IngressStalled[0][0], 0);

    let  	queue_stats = read_fabric.QueueStats();
    jeeves_assert!( ctx, queue_stats._KlEgressHighWater[0][0] >= 1);
});

//-------------------------------------------------------------------------------------------------
// Test 8: CycleTraceMechanicsAndWorkerInvariance
//
// Topology:
//   Karst(8, 8) fabric instantiated under 1-worker and 3-worker compute budgets with internal
//   cycle trace ring buffer (capacity 1024).
// Purpose:
//   Validates lazy trace history preservation across enable/disable toggles, bounds clamping at
//   capacity, trace clearing, and bit-exact cycle trace invariance between 1-worker and 3-worker fabrics.
// Implementation:
//   1. Toggles EnableCycleTrace(true/false/true), confirming history is retained and resumes at cycle 4.
//   2. Advances past 1024 cycles, confirms overflow flag, calls ClearCycleTrace, confirms reset to 0.
//   3. Runs identical write/read workloads on 1-worker and 3-worker fabrics for 32 cycles, verifying
//      cycle-by-cycle bit-exact trace equality.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, CycleTraceMechanicsAndWorkerInvariance, |ctx| {
    // 1. History preservation across enable/disable state transitions
    let  	mut fabric = KarstFabric::new();
    fabric.EnableCycleTrace( true);
    fabric.Advance( 2);
    fabric.EnableCycleTrace( false);
    fabric.Advance( 2);
    jeeves_assert_eq!( ctx, fabric.CycleTrace().Len(), 2);
    fabric.EnableCycleTrace( true);
    fabric.Advance( 2);
    jeeves_assert_eq!( ctx, fabric.CycleTrace().Len(), 4);
    jeeves_assert_eq!( ctx, fabric.CycleTrace()[2]._Cycle, 4);

    // 2. Capacity bounding and clearing
    fabric.ClearCycleTrace();
    fabric.Advance( crate::karst::K_CYCLE_TRACE_CAPACITY + 1);
    jeeves_assert_eq!( ctx, fabric.CycleTrace().Len(), crate::karst::K_CYCLE_TRACE_CAPACITY);
    jeeves_assert!( ctx, fabric.CycleTraceOverflowed());
    fabric.ClearCycleTrace();
    jeeves_assert_eq!( ctx, fabric.CycleTrace().Len(), 0);
    jeeves_assert!( ctx, !fabric.CycleTraceOverflowed());

    // 3. Cycle trace bit-exact invariance across worker budgets (1 worker vs 3 workers)
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
// Test 9: TwoDieParallelDeterminismAndPinning
//
// Topology:
//   Dual-die Karst(8, 8) fabric configured across multiple worker counts (1, 2, 3, 4, 8 workers),
//   with Hind Die 0 and Hind Die 1 pinned to dedicated execution threads.
// Purpose:
//   Proves bit-exact cycle trace parity across worker thread counts, validates dedicated die-worker
//   pinning configuration, and confirms step_cycle() multi-host determinism under full concurrency.
// Implementation:
//   1. Verifies ParallelDiesEnabled toggle on 2-worker fabric.
//   2. Compares cycle traces, link stats, and queue stats between single-worker and multi-worker
//      (2, 3, 4, 8) fabrics executing concurrent Die 0 and Die 1 transactions.
//   3. Advances 1-worker and 4-worker fabrics for 64 single cycles via step_cycle() with all 8 hosts
//      active, confirming bit-exact parity across traces and aggregate stats.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, TwoDieParallelDeterminismAndPinning, |ctx| {
    // 1. Parallel die configuration and worker pinning toggle
    let  	mut fabric = KarstFabric::with_workers( 2);
    jeeves_assert!( ctx, fabric.ParallelDiesEnabled());
    fabric.EnableParallelDies( false);
    jeeves_assert!( ctx, !fabric.ParallelDiesEnabled());
    fabric.EnableParallelDies( true);
    fabric.PostHostWrite( 0, 0, 0xDEAD_BEEF);
    fabric.Advance( 16);
    jeeves_assert_eq!( ctx, fabric.cycle_count(), 16);

    // 2. Worker budget invariance across [2, 3, 4, 8] workers
    for &workers in &[2, 3, 4, 8] {
        let  	mut single = KarstFabric::with_workers( 1);
        let  	mut multi = KarstFabric::with_workers( workers);
        single.EnableCycleTrace( true);
        multi.EnableCycleTrace( true);

        single.PostHostWrite( 0, 0, 0xAABB_CCDD);
        multi.PostHostWrite( 0, 0, 0xAABB_CCDD);
        single.PostHostWrite( 4, 0x1000, 0x1122_3344);
        multi.PostHostWrite( 4, 0x1000, 0x1122_3344);
        single.PostHostRead( 0, 0);
        multi.PostHostRead( 0, 0);
        single.PostHostRead( 4, 0x1000);
        multi.PostHostRead( 4, 0x1000);

        single.Advance( 32);
        multi.Advance( 32);

        let  	single_trace = single.CycleTrace();
        let  	multi_trace = multi.CycleTrace();
        jeeves_assert_eq!( ctx, single_trace.Len(), 32);
        jeeves_assert_eq!( ctx, multi_trace.Len(), 32);
        USeg::FromLen( single_trace.Len()).Traverse( |cycle| {
            jeeves_assert_eq!( ctx, single_trace[cycle], multi_trace[cycle]);
        });
        jeeves_assert_eq!( ctx, single.LinkStats(), multi.LinkStats());
        jeeves_assert_eq!( ctx, single.QueueStats(), multi.QueueStats());
    }

    // 3. StepCycle multi-host determinism and trace parity (8 hosts active, 64 cycles)
    let  	mut f_serial = KarstFabric::with_workers( 1);
    let  	mut f_parallel = KarstFabric::with_workers( 4);
    f_serial.EnableCycleTrace( true);
    f_parallel.EnableCycleTrace( true);

    for h in 0..K_HOSTS_PER_FABRIC {
        f_serial.PostHostWrite( h, h * 0x400, 0xCAFE_0000 + h);
        f_parallel.PostHostWrite( h, h * 0x400, 0xCAFE_0000 + h);
        f_serial.PostHostRead( h, h * 0x400);
        f_parallel.PostHostRead( h, h * 0x400);
    }
    for _ in 0..64 {
        f_serial.step_cycle();
        f_parallel.step_cycle();
    }
    let  	trace_s = f_serial.CycleTrace();
    let  	trace_p = f_parallel.CycleTrace();
    jeeves_assert_eq!( ctx, trace_s.Len(), 64);
    jeeves_assert_eq!( ctx, trace_p.Len(), 64);
    USeg::FromLen( 64).Traverse( |cycle| {
        jeeves_assert_eq!( ctx, trace_s[cycle], trace_p[cycle]);
    });
    jeeves_assert_eq!( ctx, f_serial.LinkStats(), f_parallel.LinkStats());
    jeeves_assert_eq!( ctx, f_serial.QueueStats(), f_parallel.QueueStats());
    jeeves_assert_eq!( ctx, f_serial.Stats(), f_parallel.Stats());
});

//-------------------------------------------------------------------------------------------------
// Test 10: IndependentFabricBatchExecution
//
// Topology:
//   Batch of 6 independent Karst(8, 8) fabrics executed with worker thread affinity across 3 workers
//   vs 6 serially advanced fabrics.
// Purpose:
//   Validates that parallel batch advancement with worker affinity produces bit-identical memory
//   states, cycle counts, and fabric statistics to serial execution, while respecting worker count bounds.
// Implementation:
//   Tests AdvanceIndependent worker clamping (0 workers clamped to 1, 8 workers clamped to batch size),
//   and executes 6 parallel fabrics across 3 workers for 32 cycles, verifying exact match with serial fabrics.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, IndependentFabricBatchExecution, |ctx| {
    // 1. Worker count bounds clamping
    let  	mut bounded_fabrics = [KarstFabric::with_workers( 1), KarstFabric::with_workers( 1)];
    KarstFabric::AdvanceIndependent( ( &mut bounded_fabrics).into(), 3, 0);
    KarstFabric::AdvanceIndependent( ( &mut bounded_fabrics).into(), 5, 8);
    jeeves_assert_eq!( ctx, bounded_fabrics[0].cycle_count(), 8);
    jeeves_assert_eq!( ctx, bounded_fabrics[1].cycle_count(), 8);

    // 2. Parallel batch execution with worker affinity matches serial
    let  	mut parallel = [
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
    ];
    let  	mut serial = [
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
    ];
    for i in 0..6 {
        parallel[i].PostHostWrite( 0, ( i as u32) * 4, 0x1000 + ( i as u32));
        serial[i].PostHostWrite( 0, ( i as u32) * 4, 0x1000 + ( i as u32));
    }
    KarstFabric::AdvanceIndependent( ( &mut parallel).into(), 32, 3);
    for i in 0..6 {
        serial[i].Advance( 32);
        jeeves_assert_eq!( ctx, parallel[i].cycle_count(), 32);
        jeeves_assert_eq!( ctx, parallel[i].stats(), serial[i].stats());
        jeeves_assert_eq!(
            ctx,
            parallel[i].MemChan( 0).read_word( ( i as u32) * 4).unwrap(),
            0x1000 + ( i as u32)
        );
    }
});

//-------------------------------------------------------------------------------------------------
// Test 11: IndependentFabricCoroExecution
//
// Topology:
//   Batch of 4 independent Karst(8, 8) fabrics executed via coroutine batch runner
//   (AdvanceIndependentCoro) coupled with the global Heist thread-pool scheduler (Atelier).
// Purpose:
//   Validates coroutine-based chunked execution (8 ticks per suspension) across pinned workers,
//   ensuring it preserves the global Atelier singleton, rejects zero chunk sizes, and matches serial execution.
// Implementation:
//   1. Verifies global Atelier::Instance() pointer preservation across coroutine execution.
//   2. Asserts AdvanceIndependentCoro with chunk_ticks=0 panics cleanly via catch_unwind.
//   3. Runs 4 parallel fabrics in 8-tick chunks for 32 ticks across 2 workers, verifying exact match with serial.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, IndependentFabricCoroExecution, |ctx| {
    use crate::heist::Atelier;

    // 1. Atelier global instance preservation
    let  	global = Atelier::Instance();
    let  	mut single_batch = [KarstFabric::with_workers( 1)];
    KarstFabric::AdvanceIndependentCoro( ( &mut single_batch).into(), 3, u32::MAX, 1);
    jeeves_assert_eq!( ctx, single_batch[0].Stats()._CycleCount, 3);
    jeeves_assert!( ctx, std::sync::Arc::ptr_eq( &global, &Atelier::Instance()));

    // 2. Rejection of zero chunk ticks
    let  	mut zero_batch = [KarstFabric::with_workers( 1)];
    let  	panic_res = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || {
        KarstFabric::AdvanceIndependentCoro( ( &mut zero_batch).into(), 1, 1, 0);
    }));
    jeeves_assert!( ctx, panic_res.is_err());

    // 3. Pinned coroutine resumption across workers matches serial
    let  	mut fabrics = [
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
    ];
    let  	mut serial_fabrics = [
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
        KarstFabric::with_workers( 1),
    ];
    for i in 0..4 {
        fabrics[i].PostHostWrite( 0, ( i as u32) * 4, 0xCAFE_0000 + ( i as u32));
        serial_fabrics[i].PostHostWrite( 0, ( i as u32) * 4, 0xCAFE_0000 + ( i as u32));
    }
    KarstFabric::AdvanceIndependentCoro( ( &mut fabrics).into(), 32, 2, 8);
    for i in 0..4 {
        serial_fabrics[i].Advance( 32);
        jeeves_assert_eq!( ctx, fabrics[i].cycle_count(), 32);
        jeeves_assert_eq!( ctx, fabrics[i].stats(), serial_fabrics[i].stats());
        jeeves_assert_eq!(
            ctx,
            fabrics[i].MemChan( 0).read_word( ( i as u32) * 4).unwrap(),
            0xCAFE_0000 + ( i as u32)
        );
    }
});

//-------------------------------------------------------------------------------------------------
// Test 12: VpuComputeAndBatchOrdering
//
// Topology:
//   8 near-memory VPUs (VPU 0..7) attached to 8 DDR5 memory channels (MemChan 0..7) spanning both
//   Hind dies, executing Swarm SIMT DoubleKernel kernels across 4 compute workers.
// Purpose:
//   Validates single VPU in-place memory mutation, invalid descriptor error propagation,
//   intra-channel serial ordering vs inter-channel concurrency, and full 8-channel batch scaling.
// Implementation:
//   1. Dispatches VPU 0 with DoubleKernel, verifying in-place mutation and dispatch count increment.
//   2. Submits batch with invalid 2D workgroup dimension, confirming SwarmErrorKind::ExecutionError.
//   3. Submits batch with two dispatches to chan 0 (4x) and one dispatch to chans 1..3 (2x).
//   4. Submits batch targeting all 8 memory channels simultaneously with 4 workers.
//-------------------------------------------------------------------------------------------------

jeeves_test!( Karst, VpuComputeAndBatchOrdering, |ctx| {
    // 1. Single VPU dispatch in-place mutation
    let  	mut fabric = KarstFabric::new();
    fabric.MemChanMut( 0).fill( 100);
    jeeves_assert!( ctx, fabric.MemChan( 0).verify( 100));
    let  	err = fabric.dispatch_vpu( 0, 0, WorkgroupDim::Linear( 16));
    jeeves_assert!( ctx, err.is_ok());
    jeeves_assert_eq!( ctx, fabric.VPU( 0).dispatches(), 1);
    jeeves_assert!( ctx, !fabric.MemChan( 0).verify( 100));

    // 2. Error propagation on invalid workgroup dimensions
    let  	mut fabric_err = KarstFabric::with_workers( 2);
    let  	bad_dispatches = [
        VpuDispatchDesc { _vpu_idx: 0, _chan_idx: 0, _dim: WorkgroupDim::New( 1, 2, 1) },
        VpuDispatchDesc { _vpu_idx: 1, _chan_idx: 1, _dim: WorkgroupDim::Linear( 1) },
    ];
    let  	res_err = fabric_err.dispatch_vpu_batch( &bad_dispatches);
    jeeves_assert_eq!( ctx, res_err.unwrap_err().Kind(), SwarmErrorKind::ExecutionError);
    jeeves_assert_eq!( ctx, fabric_err.vpu( 0).dispatches(), 0);

    // 3. Intra-channel serial ordering (4x) vs inter-channel concurrency (2x)
    let  	mut fabric_order = KarstFabric::with_workers( 2);
    for chan in 0..4 {
        let  	initial_val: [u32; 4] = [1, 2, 3, 4];
        let  	bytes: &[u8] = unsafe {
            std::slice::from_raw_parts( initial_val.as_ptr() as *const u8, 16)
        };
        fabric_order.MemChan_mut( chan).buffer().WriteAt( 0, bytes.into()).unwrap();
    }
    let  	dispatches = [
        VpuDispatchDesc { _vpu_idx: 0, _chan_idx: 0, _dim: WorkgroupDim::Linear( 4) },
        VpuDispatchDesc { _vpu_idx: 0, _chan_idx: 0, _dim: WorkgroupDim::Linear( 4) },
        VpuDispatchDesc { _vpu_idx: 1, _chan_idx: 1, _dim: WorkgroupDim::Linear( 4) },
        VpuDispatchDesc { _vpu_idx: 2, _chan_idx: 2, _dim: WorkgroupDim::Linear( 4) },
        VpuDispatchDesc { _vpu_idx: 3, _chan_idx: 3, _dim: WorkgroupDim::Linear( 4) },
    ];
    jeeves_assert!( ctx, fabric_order.dispatch_vpu_batch( &dispatches).is_ok());

    let  	mut out0 = [0u32; 4];
    let  	out0_bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut( out0.as_mut_ptr() as *mut u8, 16)
    };
    fabric_order.MemChan( 0).buffer().ReadAt( 0, out0_bytes.into()).unwrap();
    jeeves_assert_eq!( ctx, out0, [4, 8, 12, 16]);

    for chan in 1..4 {
        let  	mut out = [0u32; 4];
        let  	out_bytes: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut( out.as_mut_ptr() as *mut u8, 16)
        };
        fabric_order.MemChan( chan).buffer().ReadAt( 0, out_bytes.into()).unwrap();
        jeeves_assert_eq!( ctx, out, [2, 4, 6, 8]);
    }

    // 4. All 8 channels batch scaling via 4 compute workers
    let  	mut fabric_all = KarstFabric::with_workers( 4);
    for chan in 0..K_MEM_CHANS_PER_FABRIC {
        let  	initial_val: [u32; 4] = [10, 20, 30, 40];
        let  	bytes: &[u8] = unsafe {
            std::slice::from_raw_parts( initial_val.as_ptr() as *const u8, 16)
        };
        fabric_all.MemChan_mut( chan).buffer().WriteAt( 0, bytes.into()).unwrap();
    }
    let  	mut dispatches_all = [VpuDispatchDesc {
        _vpu_idx:  0,
        _chan_idx: 0,
        _dim:      WorkgroupDim::Linear( 4),
    }; K_MEM_CHANS_PER_FABRIC as usize];
    for ( c, desc) in dispatches_all.iter_mut().enumerate() {
        desc._vpu_idx = c as u32;
        desc._chan_idx = c as u32;
    }
    jeeves_assert!( ctx, fabric_all.dispatch_vpu_batch( &dispatches_all).is_ok());

    for chan in 0..K_MEM_CHANS_PER_FABRIC {
        let  	mut out = [0u32; 4];
        let  	out_bytes: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut( out.as_mut_ptr() as *mut u8, 16)
        };
        fabric_all.MemChan( chan).buffer().ReadAt( 0, out_bytes.into()).unwrap();
        jeeves_assert_eq!( ctx, out, [20, 40, 60, 80]);
    }
});

//-------------------------------------------------------------------------------------------------
// CircuitXplr trait implementation on Karst multi-die SoC fabric, dies, and interconnects
jeeves_test!( Karst, CircuitXplrHierarchyAndBuses, |ctx| {
    use crate::fenst::xplr::{CircuitXplr, Xplr};
    use crate::karst::circuit_xplr::KarstDieXplr;

    let fabric = KarstFabric::new();

    // 1. Top-Level Multi-Die SoC Fabric
    let circuit = fabric.AsCircuit();
    jeeves_assert_eq!(ctx, circuit.Name(), "KarstFabric");
    jeeves_assert_eq!(ctx, circuit.ComponentType(), "SoC Fabric (Multi-Die)");
    jeeves_assert!(ctx, circuit.Circuit().is_some());

    // Submodules: Die 0 and Die 1
    let dies = circuit.Submodules();
    jeeves_assert_eq!(ctx, dies.len(), 2);
    jeeves_assert_eq!(ctx, dies[0].Name(), "Die 0");
    jeeves_assert_eq!(ctx, dies[1].Name(), "Die 1");

    // Inter-die physical links
    let wires = circuit.Wires();
    jeeves_assert_eq!(ctx, wires.len(), 2);
    jeeves_assert!(ctx, wires.iter().any(|w| w.driver_module == "Die 0" && w.sink_module == "Die 1"));
    jeeves_assert!(ctx, wires.iter().any(|w| w.driver_module == "Die 1" && w.sink_module == "Die 0"));

    // 2. Intra-Die Circuit (Die 0)
    let die0 = KarstDieXplr::New(&fabric, 0);
    jeeves_assert_eq!(ctx, die0.Name(), "Die 0");
    jeeves_assert_eq!(ctx, die0.ComponentType(), "KarstHind Die");

    let die_subs = die0.Submodules();
    // 4 Hosts + 1 NoC + 4 MemChans + 4 VPUs = 13
    jeeves_assert_eq!(ctx, die_subs.len(), 13);
    for h in 0..4 {
        let h_name = format!("Host {}", h);
        jeeves_assert!(ctx, die_subs.iter().any(|s| s.Name() == h_name));
    }
    jeeves_assert!(ctx, die_subs.iter().any(|s| s.Name() == "NoC 0"));
    for c in 0..4 {
        let m_name = format!("MemChan {}", c);
        let v_name = format!("VPU {}", c);
        jeeves_assert!(ctx, die_subs.iter().any(|s| s.Name() == m_name));
        jeeves_assert!(ctx, die_subs.iter().any(|s| s.Name() == v_name));
    }

    // Intra-die buses (Host-NoC, NoC-MemChan, MemChan-VPU, InterDie boundary)
    let die_wires = die0.Wires();
    jeeves_assert!(ctx, die_wires.len() >= 24);
    jeeves_assert!(ctx, die_wires.iter().any(|w| w.driver_module == "Host 0" && w.sink_module == "NoC 0"));
    jeeves_assert!(ctx, die_wires.iter().any(|w| w.driver_module == "NoC 0" && w.sink_module == "MemChan 0"));
    jeeves_assert!(ctx, die_wires.iter().any(|w| w.driver_module == "MemChan 0" && w.sink_module == "VPU 0"));
});

//-------------------------------------------------------------------------------------------------
jeeves_test!( Karst, KarstFabricClassicalSchematicWithWiresAndDies, |ctx| {
    use crate::fenst::cask::CaskRenderCommand;
    use crate::fenst::cask2d::Cask2dColorScheme;
    use crate::karst::circuit_xplr::KarstDieXplr;
    use crate::karst::fabric::KarstFabric;
    use crate::karst::scene;
    use crate::silo::IArr;
    use std::sync::atomic::AtomicBool;

    let mut fabric  = KarstFabric::new();
    fabric.post_host_write( 0, 0x100, 42);
    fabric.step_cycle();

    let colors = Cask2dColorScheme::default();

    // 1. Classical 2D Top-Level Fabric Schematic (Multi-Die SoC)
    let fabric_schematic = scene::FabricSchematic( &fabric, colors.clone());

    // Root (Fabric) + 2 Dies = 3 nodes
    jeeves_assert_eq!( ctx, fabric_schematic.NodeCount(), 3);
    jeeves_assert_eq!( ctx, fabric_schematic.Nodes()[0].Name(), "KarstFabric");

    // Dual-die layout verification: Die 0 (left) vs Die 1 (right)
    let die0_node   = fabric_schematic.Nodes().iter().find( |n| n.Name() == "Die 0").unwrap();
    let die1_node   = fabric_schematic.Nodes().iter().find( |n| n.Name() == "Die 1").unwrap();
    jeeves_assert!( ctx, die0_node.Bounds().x < die1_node.Bounds().x, "Die 0 must be positioned to the left of Die 1");

    // Check inter-die wires
    let mut fabric_lines = 0;
    for cmd in fabric_schematic.Commands()
    {
        if matches!( cmd, CaskRenderCommand::Line { .. })
        {
            fabric_lines += 1;
        }
    }
    jeeves_assert!( ctx, fabric_lines >= 2, "Expected inter-die physical link lines");

    // 2. Classical 2D Die Schematic (Intra-Die: Hosts, NoC, MemChans, VPUs)
    let die0_circuit = KarstDieXplr::New( &fabric, 0);
    let die0_schematic = scene::DieSchematic( &die0_circuit, colors);

    // Root (Die 0) + 4 Hosts + 1 NoC + 4 MemChans + 4 VPUs = 14 nodes
    jeeves_assert_eq!( ctx, die0_schematic.NodeCount(), 14);
    jeeves_assert_eq!( ctx, die0_schematic.Nodes()[0].Name(), "Die 0");

    let host0 = die0_schematic.Nodes().iter().find( |n| n.Name() == "Host 0").unwrap();
    let noc0 = die0_schematic.Nodes().iter().find( |n| n.Name() == "NoC 0").unwrap();
    let mc0 = die0_schematic.Nodes().iter().find( |n| n.Name() == "MemChan 0").unwrap();
    let vpu0 = die0_schematic.Nodes().iter().find( |n| n.Name() == "VPU 0").unwrap();

    // Topological staging: Hosts (left) -> NoC -> MemChans -> VPUs (right)
    jeeves_assert!( ctx, host0.Bounds().x < noc0.Bounds().x, "Host must be left of NoC");
    jeeves_assert!( ctx, noc0.Bounds().x < mc0.Bounds().x, "NoC must be left of MemChan");
    jeeves_assert!( ctx, mc0.Bounds().x < vpu0.Bounds().x, "MemChan must be left of VPU");

    let mut die_lines = 0;
    for cmd in die0_schematic.Commands()
    {
        if matches!( cmd, CaskRenderCommand::Line { .. })
        {
            die_lines += 1;
        }
    }
    // Multiple buses connecting Host <-> NoC <-> MemChan <-> VPU
    jeeves_assert!( ctx, die_lines >= 20, "Expected intra-die interconnect buses");

    // 3. 3D Karst Fabric Hierarchy with Components, Ports, and Wires
    let fabric_scene = scene::FabricScene( &fabric, &AtomicBool::new( false)).unwrap();
    jeeves_assert!( ctx, fabric_scene.Nodes().Size() >= 30, "Fabric scene should contain full SoC hierarchy");
    let mut found_noc = false;
    let mut found_link = false;
    fabric_scene.Nodes().Traverse( |node| {
        if node.Name().contains( "NoC")
        {
            found_noc = true;
        }
        if node.Name().contains( "InterDie")
        {
            found_link = true;
        }
    });
    jeeves_assert!( ctx, found_noc, "Fabric scene should contain NoC router volumes");
    jeeves_assert!( ctx, found_link, "Fabric scene should contain inter-die link volume");
});
