//-- hub.rs ---------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::crew::config::CrewLinkConfig;
use	crate::crew::node::{ CrewNode, NodeStats };
use	crate::crew::protocol::{ CoSimAction, ProtocolMessage, REG_NODE_ID, REG_RX_COUNT, REG_RX_DATA, REG_STATUS, REG_TX_DATA, STATUS_PEER_UP, STATUS_RX_READY, STATUS_TX_READY };
use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	crate::silo::useg::USeg;
use	crate::stalks::work::SpinMutex;
use	std::sync::Arc;

//--------------------------------------------------------------------------------------------------
/// Callback type for monitoring byte-level routing between VM nodes.
pub type MessageCallback = Arc< dyn Fn( u32, u32, u8) + Send + Sync>;

//--------------------------------------------------------------------------------------------------
/// Concrete co-simulation hub managing VM nodes and in-memory protocol dispatch.
pub struct CrewHub
{
    _Nodes: SpinMutex< Stash< Arc< CrewNode>>>,
    _MessageCb: SpinMutex< Option< MessageCallback>>,
    _Routes: SpinMutex< Stash< ( u32, Buff< u32>)>>,
}
impl Default for CrewHub {
    fn	default() -> Self
    {
        Self::New()
    }
}
impl CrewHub
{
    pub fn	New() -> Self
    {
        Self {
            _Nodes: SpinMutex::New( Stash::New()),
            _MessageCb: SpinMutex::New( None),
            _Routes: SpinMutex::New( Stash::New()),
        }
    }
    #[inline]
    pub fn	new() -> Self
    {
        Self::New()
    }
    pub fn	AddLink( &self, config: CrewLinkConfig)
    {
        let  	mut routes = self._Routes.Lock();
        let  	mut found = false;
        let  	src = config.source_node_id;
        USeg::FromLen( routes.Size()).Traverse( |i| {
            if routes[i].0 == src {
                found = true;
            }
        });
        if !found {
            routes.Push( ( src, config.destination_node_ids));
        }
    }
    #[inline]
    pub fn	add_link( &self, config: CrewLinkConfig)
    {
        self.AddLink( config);
    }
    pub fn	AddNode( &self, id: u32)
    {
        let  	mut nodes = self._Nodes.Lock();
        nodes.Push( Arc::new( CrewNode::New( id)));
    }
    #[inline]
    pub fn	add_node( &self, id: u32)
    {
        self.AddNode( id);
    }
    pub fn	NodeCount( &self) -> u32
    {
        let  	nodes = self._Nodes.Lock();
        nodes.Size()
    }
    #[inline]
    pub fn	node_count( &self) -> u32
    {
        self.NodeCount()
    }
    pub fn	FindNode( &self, id: u32) -> Option< Arc< CrewNode>>
    {
        let  	nodes = self._Nodes.Lock();
        let  	mut found: Option< Arc< CrewNode>> = None;
        USeg::FromLen( nodes.Size()).Traverse( |i| {
            if found.is_none() && nodes[i].Id() == id {
                found = Some( nodes[i].clone());
            }
        });
        found
    }
    #[inline]
    pub fn	find_node( &self, id: u32) -> Option< Arc< CrewNode>>
    {
        self.FindNode( id)
    }
    pub fn	IsNodeOnline( &self, id: u32) -> bool
    {
        if let  	Some( node) = self.FindNode( id) {
            node.IsOnline()
        } else {
            false
        }
    }
    #[inline]
    pub fn	is_node_online( &self, id: u32) -> bool
    {
        self.IsNodeOnline( id)
    }
    pub fn	GetNodeStats( &self, id: u32) -> NodeStats
    {
        if let  	Some( node) = self.FindNode( id) {
            node.GetStats()
        } else {
            NodeStats::default()
        }
    }
    #[inline]
    pub fn	get_node_stats( &self, id: u32) -> NodeStats
    {
        self.GetNodeStats( id)
    }
    pub fn	SetMessageCallback< F>( &self, cb: F)
    where
        F: Fn( u32, u32, u8) + Send + Sync + 'static,
    {
        let  	mut cbGuard = self._MessageCb.Lock();
        *cbGuard = Some( Arc::new( cb));
    }
    #[inline]
    pub fn	set_message_callback< F>( &self, cb: F)
    where
        F: Fn( u32, u32, u8) + Send + Sync + 'static,
    {
        self.SetMessageCallback( cb);
    }
    pub fn	HandleRequest( &self, node: &CrewNode, req: &ProtocolMessage) -> ProtocolMessage
    {
        let  	mut resp = ProtocolMessage {
            _ActionId: CoSimAction::Ok as i32,
            _Addr: req.Addr(),
            _Value: 0,
            _PeripheralIndex: req.PeripheralIndex(),
        };
        let  	reg = ( req.Addr() & 0xFFF) as u32;
        let  	action = req.Action();
        match action {
            CoSimAction::ReadBus
            | CoSimAction::ReadBusByte
            | CoSimAction::ReadBusWord
            | CoSimAction::ReadBusDword
            | CoSimAction::ReadBusQword => {
                node.RecordRead();
                match reg {
                    REG_NODE_ID => {
                        resp._Value = node.Id() as u64;
                    }
                    REG_STATUS => {
                        let  	mut status = 0;
                        if node.RxCount() > 0 {
                            status |= STATUS_RX_READY;
                        }
                        let  	peers = self.getPeersForNode( node.Id());
                        let  	mut allReady = !peers.IsEmpty();
                        let  	mut anyOnline = false;
                        USeg::FromLen( peers.Len()).Traverse( |i| {
                            if let  	Some( peer) = self.FindNode( peers[i]) {
                                if peer.IsOnline() {
                                    anyOnline = true;
                                } else {
                                    allReady = false;
                                }
                                if !peer.CanPushRx() {
                                    allReady = false;
                                }
                            } else {
                                allReady = false;
                            }
                        });
                        if allReady {
                            status |= STATUS_TX_READY;
                        }
                        if anyOnline {
                            status |= STATUS_PEER_UP;
                        }
                        resp._Value = status as u64;
                    }
                    REG_TX_DATA => {
                        resp._Value = 0;
                    }
                    REG_RX_DATA => {
                        let  	mut byte = 0u8;
                        if node.PopRx( &mut byte) {
                            resp._Value = byte as u64;
                        } else {
                            resp._Value = 0;
                        }
                    }
                    REG_RX_COUNT => {
                        resp._Value = node.RxCount() as u64;
                    }
                    _ => {
                        resp._Value = 0;
                    }
                }
            }
            CoSimAction::WriteBusByte
            | CoSimAction::WriteBusWord
            | CoSimAction::WriteBusDword
            | CoSimAction::WriteBusQword => {
                node.RecordWrite();
                if reg == REG_TX_DATA && action == CoSimAction::WriteBusByte {
                    let  	byte = ( req.Value() & 0xFF) as u8;
                    let  	peers = self.getPeersForNode( node.Id());
                    let  	mut accepted = !peers.IsEmpty();
                    USeg::FromLen( peers.Len()).Traverse( |i| {
                        let  	peerId = peers[i];
                        if let  	Some( peer) = self.FindNode( peerId) {
                            if !peer.IsOnline() || !peer.CanPushRx() {
                                accepted = false;
                            }
                        } else {
                            accepted = false;
                        }
                    });
                    if !accepted {
                        resp._ActionId = CoSimAction::Error as i32;
                        return resp;
                    }
                    node.RecordByteSent();
                    let  	mut allDelivered = true;
                    USeg::FromLen( peers.Len()).Traverse( |i| {
                        let  	peerId = peers[i];
                        if let  	Some( peer) = self.FindNode( peerId) {
                            if peer.TryPushRx( byte) != crate::crew::node::RxPushOutcome::Delivered {
                                allDelivered = false;
                            } else {
                                let  	cbOpt = {
                                    let  	guard = self._MessageCb.Lock();
                                    guard.clone()
                                };
                                if let  	Some( cb) = cbOpt {
                                    cb( node.Id(), peerId, byte);
                                }
                            }
                        }
                    });
                    if !allDelivered {
                        resp._ActionId = CoSimAction::Error as i32;
                    }
                } else {
                    resp._ActionId = CoSimAction::Error as i32;
                }
            }
            CoSimAction::ResetPeripheral => {
                node.ClearRx();
            }
            _ => {}
        }
        resp
    }
    #[inline]
    pub fn	handle_request( &self, node: &CrewNode, req: &ProtocolMessage) -> ProtocolMessage
    {
        self.HandleRequest( node, req)
    }
    fn	getPeersForNode( &self, nodeId: u32) -> Buff< u32>
    {
        let  	routes = self._Routes.Lock();
        let  	mut dests = Buff::New();
        USeg::FromLen( routes.Size()).Traverse( |i| {
            if routes[i].0 == nodeId {
                dests = routes[i].1.clone();
            }
        });
        if dests.IsEmpty() {
            // Trellis default: peerId = 1 - node.Id()
            let  	peerId = if nodeId == 0 { 1 } else { 0 };
            if self.FindNode( peerId).is_some() {
                dests = Buff::FromDispenser( 1, |_| peerId);
            }
        }
        dests
    }
}
