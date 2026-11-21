//-- hub.rs ---------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::crew::config::CrewLinkConfig;
use	crate::crew::node::{ CrewNode, NodeStats, RxPushOutcome };
use	crate::crew::protocol::{ CoSimAction, ProtocolMessage, REG_NODE_ID, REG_RX_COUNT, REG_RX_DATA, REG_STATUS, REG_TX_DATA, STATUS_PEER_UP, STATUS_RX_READY, STATUS_TX_READY };
use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	crate::silo::useg::USeg;
use	crate::stalks::work::SpinMutex;

//--------------------------------------------------------------------------------------------------
/// Callback type for monitoring byte-level routing between VM nodes.
pub type MessageCallback = Box< dyn Fn( u32, u32, u8) + Send + Sync>;

//--------------------------------------------------------------------------------------------------
/// Conversion trait to extract node ID from handles, raw IDs, or CrewNode references.
pub trait AsNodeId
{
    fn	AsNodeId( &self) -> u32;
}

impl AsNodeId for u32
{
    #[inline]
    fn	AsNodeId( &self) -> u32
    {
        *self
    }
}

impl AsNodeId for CrewNodeHandle
{
    #[inline]
    fn	AsNodeId( &self) -> u32
    {
        self._id
    }
}

impl AsNodeId for CrewNode
{
    #[inline]
    fn	AsNodeId( &self) -> u32
    {
        self.Id()
    }
}

//--------------------------------------------------------------------------------------------------
/// Lightweight handle referencing a pinned CrewNode inside a CrewHub.
#[derive( Copy, Clone)]
pub struct CrewNodeHandle
{
    _node: *const CrewNode,
    _id:   u32,
}

unsafe impl Send for CrewNodeHandle {}
unsafe impl Sync for CrewNodeHandle {}

impl CrewNodeHandle
{
    #[inline]
    fn	Node( &self) -> &CrewNode
    {
        unsafe { &*self._node }
    }

    #[inline]
    pub fn	Id( &self) -> u32
    {
        self._id
    }

    #[inline]
    pub fn	id( &self) -> u32
    {
        self._id
    }

    #[inline]
    pub fn	IsOnline( &self) -> bool
    {
        self.Node().IsOnline()
    }

    #[inline]
    pub fn	is_online( &self) -> bool
    {
        self.Node().is_online()
    }

    #[inline]
    pub fn	SetOnline( &self, online: bool)
    {
        self.Node().SetOnline( online);
    }

    #[inline]
    pub fn	set_online( &self, online: bool)
    {
        self.Node().set_online( online);
    }

    #[inline]
    pub fn	PushRx( &self, byte: u8) -> bool
    {
        self.Node().PushRx( byte)
    }

    #[inline]
    pub fn	push_rx( &self, byte: u8) -> bool
    {
        self.Node().push_rx( byte)
    }

    #[inline]
    pub fn	TryPushRx( &self, byte: u8) -> RxPushOutcome
    {
        self.Node().TryPushRx( byte)
    }

    #[inline]
    pub fn	try_push_rx( &self, byte: u8) -> RxPushOutcome
    {
        self.Node().TryPushRx( byte)
    }

    #[inline]
    pub fn	PopRx( &self, byte: &mut u8) -> bool
    {
        self.Node().PopRx( byte)
    }

    #[inline]
    pub fn	pop_rx( &self, byte: &mut u8) -> bool
    {
        self.Node().pop_rx( byte)
    }

    #[inline]
    pub fn	RxCount( &self) -> u32
    {
        self.Node().RxCount()
    }

    #[inline]
    pub fn	rx_count( &self) -> u32
    {
        self.Node().rx_count()
    }

    #[inline]
    pub fn	CanPushRx( &self) -> bool
    {
        self.Node().CanPushRx()
    }

    #[inline]
    pub fn	can_push_rx( &self) -> bool
    {
        self.Node().CanPushRx()
    }

    #[inline]
    pub fn	ClearRx( &self)
    {
        self.Node().ClearRx();
    }

    #[inline]
    pub fn	clear_rx( &self)
    {
        self.Node().clear_rx();
    }

    #[inline]
    pub fn	GetStats( &self) -> NodeStats
    {
        self.Node().GetStats()
    }

    #[inline]
    pub fn	get_stats( &self) -> NodeStats
    {
        self.Node().get_stats()
    }

    #[inline]
    pub fn	ResetStats( &self)
    {
        self.Node().ResetStats();
    }

    #[inline]
    pub fn	reset_stats( &self)
    {
        self.Node().reset_stats();
    }
}

//--------------------------------------------------------------------------------------------------
/// Concrete co-simulation hub managing VM nodes and in-memory protocol dispatch.
pub struct CrewHub
{
    _Nodes:     SpinMutex< Stash< Box< CrewNode>>>,
    _MessageCb: SpinMutex< Option< MessageCallback>>,
    _Routes:    SpinMutex< Stash< ( u32, Buff< u32>)>>,
}

impl Default for CrewHub
{
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
            _Nodes:     SpinMutex::New( Stash::New()),
            _MessageCb: SpinMutex::New( None),
            _Routes:    SpinMutex::New( Stash::New()),
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
            if routes[i].0 == src
            {
                found = true;
            }
        });
        if !found
        {
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
        nodes.Push( Box::new( CrewNode::New( id)));
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

    pub fn	HasNode( &self, id: u32) -> bool
    {
        let  	nodes = self._Nodes.Lock();
        let  	mut found = false;
        USeg::FromLen( nodes.Size()).Span( |i| {
            if nodes[i].Id() == id
            {
                found = true;
                return false;
            }
            true
        });
        found
    }

    pub fn	FindNode( &self, id: u32) -> Option< CrewNodeHandle>
    {
        let  	nodes = self._Nodes.Lock();
        let  	mut found: Option< CrewNodeHandle> = None;
        USeg::FromLen( nodes.Size()).Span( |i| {
            if nodes[i].Id() == id
            {
                found = Some( CrewNodeHandle {
                    _node: ( &*nodes[i]) as *const CrewNode,
                    _id:   id,
                });
                return false;
            }
            true
        });
        found
    }

    #[inline]
    pub fn	find_node( &self, id: u32) -> Option< CrewNodeHandle>
    {
        self.FindNode( id)
    }

    pub fn	IsNodeOnline( &self, id: u32) -> bool
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.IsOnline()
        }
        else
        {
            false
        }
    }

    #[inline]
    pub fn	is_node_online( &self, id: u32) -> bool
    {
        self.IsNodeOnline( id)
    }

    pub fn	SetNodeOnline( &self, id: u32, online: bool)
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.SetOnline( online);
        }
    }

    #[inline]
    pub fn	set_node_online( &self, id: u32, online: bool)
    {
        self.SetNodeOnline( id, online);
    }

    pub fn	PushNodeRx( &self, id: u32, byte: u8) -> bool
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.PushRx( byte)
        }
        else
        {
            false
        }
    }

    #[inline]
    pub fn	push_node_rx( &self, id: u32, byte: u8) -> bool
    {
        self.PushNodeRx( id, byte)
    }

    pub fn	TryPushNodeRx( &self, id: u32, byte: u8) -> RxPushOutcome
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.TryPushRx( byte)
        }
        else
        {
            RxPushOutcome::Offline
        }
    }

    #[inline]
    pub fn	try_push_node_rx( &self, id: u32, byte: u8) -> RxPushOutcome
    {
        self.TryPushNodeRx( id, byte)
    }

    pub fn	PopNodeRx( &self, id: u32, byte: &mut u8) -> bool
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.PopRx( byte)
        }
        else
        {
            false
        }
    }

    #[inline]
    pub fn	pop_node_rx( &self, id: u32, byte: &mut u8) -> bool
    {
        self.PopNodeRx( id, byte)
    }

    pub fn	NodeRxCount( &self, id: u32) -> u32
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.RxCount()
        }
        else
        {
            0
        }
    }

    #[inline]
    pub fn	node_rx_count( &self, id: u32) -> u32
    {
        self.NodeRxCount( id)
    }

    pub fn	CanNodePushRx( &self, id: u32) -> bool
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.CanPushRx()
        }
        else
        {
            false
        }
    }

    #[inline]
    pub fn	can_node_push_rx( &self, id: u32) -> bool
    {
        self.CanNodePushRx( id)
    }

    pub fn	ClearNodeRx( &self, id: u32)
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.ClearRx();
        }
    }

    #[inline]
    pub fn	clear_node_rx( &self, id: u32)
    {
        self.ClearNodeRx( id);
    }

    pub fn	GetNodeStats( &self, id: u32) -> NodeStats
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.GetStats()
        }
        else
        {
            NodeStats::default()
        }
    }

    #[inline]
    pub fn	get_node_stats( &self, id: u32) -> NodeStats
    {
        self.GetNodeStats( id)
    }

    pub fn	ResetNodeStats( &self, id: u32)
    {
        if let  	Some( node) = self.FindNode( id)
        {
            node.ResetStats();
        }
    }

    #[inline]
    pub fn	reset_node_stats( &self, id: u32)
    {
        self.ResetNodeStats( id);
    }

    pub fn	SetMessageCallback< F>( &self, cb: F)
    where
        F: Fn( u32, u32, u8) + Send + Sync + 'static,
    {
        let  	mut cbGuard = self._MessageCb.Lock();
        *cbGuard = Some( Box::new( cb));
    }

    #[inline]
    pub fn	set_message_callback< F>( &self, cb: F)
    where
        F: Fn( u32, u32, u8) + Send + Sync + 'static,
    {
        self.SetMessageCallback( cb);
    }

    pub fn	GetPeerNodes( &self, nodeId: u32) -> Buff< CrewNodeHandle>
    {
        let  	routes = self._Routes.Lock();
        let  	mut dest_ids: Option< Buff< u32>> = None;
        USeg::FromLen( routes.Size()).Traverse( |i| {
            if routes[i].0 == nodeId
            {
                dest_ids = Some( routes[i].1.clone());
            }
        });
        drop( routes);

        let  	nodes = self._Nodes.Lock();
        let  	mut peers = Stash::New();
        match dest_ids
        {
            Some( dests) => {
                USeg::FromLen( dests.Size()).Traverse( |d_idx| {
                    let  	dest_id = dests[d_idx];
                    USeg::FromLen( nodes.Size()).Traverse( |n_idx| {
                        if nodes[n_idx].Id() == dest_id
                        {
                            peers.Push( CrewNodeHandle {
                                _node: ( &*nodes[n_idx]) as *const CrewNode,
                                _id:   dest_id,
                            });
                        }
                    });
                });
            }
            None => {
                let  	default_peer_id = if nodeId == 0 { 1 } else { 0 };
                USeg::FromLen( nodes.Size()).Traverse( |n_idx| {
                    if nodes[n_idx].Id() == default_peer_id
                    {
                        peers.Push( CrewNodeHandle {
                            _node: ( &*nodes[n_idx]) as *const CrewNode,
                            _id:   default_peer_id,
                        });
                    }
                });
            }
        }
        peers.IntoBuff()
    }

    pub fn	HandleRequest( &self, node: &impl AsNodeId, req: &ProtocolMessage) -> ProtocolMessage
    {
        let  	nodeId = node.AsNodeId();
        let  	mut resp = ProtocolMessage {
            _ActionId:        CoSimAction::Ok as i32,
            _Addr:            req.Addr(),
            _Value:           0,
            _PeripheralIndex: req.PeripheralIndex(),
        };
        let  	reg = ( req.Addr() & 0xFFF) as u32;
        let  	action = req.Action();
        let  	Some( nodeHandle) = self.FindNode( nodeId) else {
            resp._ActionId = CoSimAction::Error as i32;
            return resp;
        };
        match action
        {
            CoSimAction::ReadBus
            | CoSimAction::ReadBusByte
            | CoSimAction::ReadBusWord
            | CoSimAction::ReadBusDword
            | CoSimAction::ReadBusQword => {
                nodeHandle.Node().RecordRead();
                match reg
                {
                    REG_NODE_ID => {
                        resp._Value = nodeId as u64;
                    }
                    REG_STATUS => {
                        let  	mut status = 0;
                        if nodeHandle.RxCount() > 0
                        {
                            status |= STATUS_RX_READY;
                        }
                        let  	peers = self.GetPeerNodes( nodeId);
                        let  	mut allReady = !peers.IsEmpty();
                        let  	mut anyOnline = false;
                        peers.Traverse( |peer| {
                            if peer.IsOnline()
                            {
                                anyOnline = true;
                            }
                            else
                            {
                                allReady = false;
                            }
                            if !peer.CanPushRx()
                            {
                                allReady = false;
                            }
                        });
                        if allReady
                        {
                            status |= STATUS_TX_READY;
                        }
                        if anyOnline
                        {
                            status |= STATUS_PEER_UP;
                        }
                        resp._Value = status as u64;
                    }
                    REG_TX_DATA => {
                        resp._Value = 0;
                    }
                    REG_RX_DATA => {
                        let  	mut byte = 0u8;
                        if nodeHandle.PopRx( &mut byte)
                        {
                            resp._Value = byte as u64;
                        }
                        else
                        {
                            resp._Value = 0;
                        }
                    }
                    REG_RX_COUNT => {
                        resp._Value = nodeHandle.RxCount() as u64;
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
                nodeHandle.Node().RecordWrite();
                if reg == REG_TX_DATA && action == CoSimAction::WriteBusByte
                {
                    let  	byte = ( req.Value() & 0xFF) as u8;
                    let  	peers = self.GetPeerNodes( nodeId);
                    let  	mut accepted = !peers.IsEmpty();
                    peers.Traverse( |peer| {
                        if !peer.IsOnline() || !peer.CanPushRx()
                        {
                            accepted = false;
                        }
                    });
                    if !accepted
                    {
                        resp._ActionId = CoSimAction::Error as i32;
                        return resp;
                    }
                    nodeHandle.Node().RecordByteSent();
                    let  	mut allDelivered = true;
                    peers.Traverse( |peer| {
                        if peer.TryPushRx( byte) != RxPushOutcome::Delivered
                        {
                            allDelivered = false;
                        }
                    });
                    if !allDelivered
                    {
                        resp._ActionId = CoSimAction::Error as i32;
                    }
                    else if let Some( ref cb) = *self._MessageCb.Lock()
                    {
                        peers.Traverse( |peer| {
                            cb( nodeId, peer.Id(), byte);
                        });
                    }
                }
                else
                {
                    resp._ActionId = CoSimAction::Error as i32;
                }
            }
            CoSimAction::ResetPeripheral => {
                nodeHandle.ClearRx();
            }
            _ => {}
        }
        resp
    }

    #[inline]
    pub fn	handle_request( &self, node: &impl AsNodeId, req: &ProtocolMessage) -> ProtocolMessage
    {
        self.HandleRequest( node, req)
    }
}

//--------------------------------------------------------------------------------------------------
