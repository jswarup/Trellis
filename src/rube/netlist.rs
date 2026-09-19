//-- netlist.rs -----------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------------

use	crate::rube::port::{ PortId, PortType };
use	crate::rube::trigger::TriggerId;
use	crate::silo::{ Buff, DisjointSet, Stash, USeg };

//------------------------------------------------------------------------------------------------------------------
/// Netlist — manages port connectivity and trigger ID mapping via union-find DisjointSet.
/// Modeled directly from Trellis `netlist.h`.
pub struct Netlist
{
    pub _Equiv:         DisjointSet,
    pub _Driver:        Stash< PortId>,
    pub _RootTrigger:   Stash< TriggerId>,
    pub _NextTriggerId: u32,
    pub _TriggerTypes:  Stash< PortType>,
}
impl Default for Netlist {
    #[inline]
    fn	default() -> Self
    {
        Self::New()
    }
}
impl Netlist
{
    pub fn	New() -> Self
    {
        Self {
            _Equiv:         DisjointSet::New(),
            _Driver:        Stash::New(),
            _RootTrigger:   Stash::New(),
            _NextTriggerId: 0,
            _TriggerTypes:  Stash::New(),
        }
    }
    pub fn	Grow( &mut self, count: u32)
    {
        self._Equiv.Grow( count);
        USeg::FromLen( count).Traverse( |_| {
            self._Driver.Push( PortId::Invalid());
            self._RootTrigger.Push( u32::MAX);
        });
    }
    #[inline]
    pub fn	FindRootConst( &self, port: PortId) -> u32
    {
        self._Equiv.FindConst( port.Index())
    }
    #[inline]
    pub fn	FindRoot( &mut self, port: PortId) -> u32
    {
        self._Equiv.Find( port.Index())
    }
    pub fn	Connect( &mut self, driver: PortId, sink: PortId) -> bool
    {
        let  	sinkIdx = sink.Index();
        let  	existingDriver = self._Driver[sinkIdx];
        if existingDriver.IsValid() && existingDriver != driver {
            return false;
        }
        self._Driver[sinkIdx] = driver;
        self._Equiv.Union( driver.Index(), sink.Index());
        true
    }
    #[inline]
    pub fn	DriverOf( &mut self, port: PortId) -> PortId
    {
        let  	root = self.FindRoot( port);
        self._Driver[root]
    }
    pub fn	AssignTrigger( &mut self, rootIdx: u32, portType: PortType) -> TriggerId
    {
        let  	actualRoot = self._Equiv.Find( rootIdx);
        let  	existing = self._RootTrigger[actualRoot];
        if existing != u32::MAX {
            return existing;
        }
        let  	trigId = self._NextTriggerId;
        self._NextTriggerId += 1;
        self._RootTrigger[actualRoot] = trigId;
        self._TriggerTypes.Push( portType);
        trigId
    }
    #[inline]
    pub fn	TriggerOf( &mut self, port: PortId) -> TriggerId
    {
        let  	root = self.FindRoot( port);
        self._RootTrigger[root]
    }
    #[inline]
    pub fn	HasTrigger( &mut self, port: PortId) -> bool
    {
        let  	root = self.FindRoot( port);
        self._RootTrigger[root] != u32::MAX
    }
    #[inline]
    pub fn	HasTriggerConst( &self, port: PortId) -> bool
    {
        let  	root = self.FindRootConst( port);
        self._RootTrigger[root] != u32::MAX
    }
    pub fn	BuildPortToTrigger( &mut self) -> Buff< TriggerId>
    {
        let  	count = self._Equiv.Size();
        let  	mut portToTrigger = Stash::WithCapacity( count);
        USeg::FromLen( count).Traverse( |i| {
            let  	root = self._Equiv.Find( i);
            let  	trig = self._RootTrigger[root];
            assert!( 
                trig != u32::MAX,
                "Port index was not assigned a TriggerId before build"
            );
            portToTrigger.Push( trig);
        });
        portToTrigger.ExtractBuff()
    }
    pub fn	BuildPortToTriggerConst( &self) -> Buff< TriggerId>
    {
        let  	count = self._Equiv.Size();
        let  	mut portToTrigger = Stash::WithCapacity( count);
        USeg::FromLen( count).Traverse( |i| {
            let  	root = self._Equiv.FindConst( i);
            let  	trig = self._RootTrigger[root];
            assert!( 
                trig != u32::MAX,
                "Port index was not assigned a TriggerId before build"
            );
            portToTrigger.Push( trig);
        });
        portToTrigger.ExtractBuff()
    }
    #[inline]
    pub const fn	TriggerCount( &self) -> u32
    {
        self._NextTriggerId
    }
    #[inline]
    pub fn	TriggerType( &self, trigId: TriggerId) -> PortType
    {
        self._TriggerTypes[trigId]
    }
}
