//-- layout.rs ------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------------

use	crate::rube::coro_kernel::{ CoroCell, CoroInstance, CoroWarp };
use	crate::rube::module::{ FastWarp, KernelKind, Module };
use	crate::rube::netlist::Netlist;
use	crate::rube::port::{ ModuleId, PortDesc, PortId };
use	crate::rube::trigger::{ TriggerId, TriggerWad };
use	crate::silo::{ Arr, Buff, IArr, Stash, USeg };
use	std::sync::Arc;

//------------------------------------------------------------------------------------------------------------------
/// Layout — manages module hierarchy, netlist connections, freezing, and warp compilation.
/// Modeled directly from Trellis `layout.h`.
pub struct Layout
{
    pub _Modules: Stash< Module>,
    pub _Ports: Stash< PortDesc>,
    pub _Netlist: Netlist,
    pub _ModuleChildren: Stash< Stash< ModuleId>>,
    pub _SubModules: Stash< ModuleId>,
    pub _Descendents: Stash< ModuleId>,
    pub _PortToTrigger: Buff< TriggerId>,
}
impl Default for Layout {
    #[inline]
    fn	default() -> Self
    {
        Self::New()
    }
}
impl Layout
{
    pub fn	New() -> Self
    {
        Self {
            _Modules: Stash::New(),
            _Ports: Stash::New(),
            _Netlist: Netlist::New(),
            _ModuleChildren: Stash::New(),
            _SubModules: Stash::New(),
            _Descendents: Stash::New(),
            _PortToTrigger: Buff::New(),
        }
    }
    pub fn	AddPorts< 'a>(
        &mut self, modId: ModuleId, moduleName: &str, ports: impl Into< Arr< 'a, PortDesc>>,
    ) -> USeg
    {
        let  	arr = ports.into();
        let  	start = self._Ports.Size();
        let  	count = arr.Size();
        arr.Traverse( |item| {
            let  	mut desc = item.clone();
            desc.SetName( format!( "{}.{}", moduleName, desc.Name()));
            desc.SetOwner( modId);
            self._Ports.Push( desc);
        });
        self._Netlist.Grow( count);
        USeg::WithLen( start, count)
    }
    pub fn	AddModule< 'a>(
        &mut self, name: &str, parent: ModuleId, inPorts: impl Into< Arr< 'a, PortDesc>>,
        outPorts: impl Into< Arr< 'a, PortDesc>>, kernel: KernelKind,
    ) -> ModuleId
    {
        let  	modId = ModuleId::New( self._Modules.Size());
        let  	inSeg = self.AddPorts( modId, name, inPorts);
        let  	outSeg = self.AddPorts( modId, name, outPorts);
        let  	module = Module::New( modId, parent, name, inSeg, outSeg, kernel);
        self._Modules.Push( module);
        self._ModuleChildren.Push( Stash::New());
        if parent.IsValid() {
            assert!( 
                parent.Id() < self._Modules.Size(),
                "Parent ModuleId out of bounds"
            );
            self._ModuleChildren[parent.Id()].Push( modId);
        }
        modId
    }
    pub fn	AddCoroModule< 'a>(
        &mut self, name: &str, parent: ModuleId, inPorts: impl Into< Arr< 'a, PortDesc>>,
        outPorts: impl Into< Arr< 'a, PortDesc>>,
        factory: impl Fn() -> CoroInstance + Send + Sync + 'static,
    ) -> ModuleId
    {
        self.AddModule( 
            name,
            parent,
            inPorts,
            outPorts,
            KernelKind::Coro( Arc::new( factory)),
        )
    }
    #[inline]
    pub fn	InPort( &self, moduleId: ModuleId, portIdx: u32) -> PortId
    {
        assert!( 
            moduleId.Id() < self._Modules.Size(),
            "ModuleId out of bounds"
        );
        let  	module = &self._Modules[moduleId.Id()];
        assert!( portIdx < module._InPorts.Size(), "Port index out of bounds");
        PortId::In( module._InPorts.First() + portIdx)
    }
    #[inline]
    pub fn	OutPort( &self, moduleId: ModuleId, portIdx: u32) -> PortId
    {
        assert!( 
            moduleId.Id() < self._Modules.Size(),
            "ModuleId out of bounds"
        );
        let  	module = &self._Modules[moduleId.Id()];
        assert!( 
            portIdx < module._OutPorts.Size(),
            "Port index out of bounds"
        );
        PortId::Out( module._OutPorts.First() + portIdx)
    }
    pub fn	Connect( &mut self, src: PortId, dst: PortId) -> &mut Self
    {
        let  	srcIdx = src.Index();
        let  	dstIdx = dst.Index();
        assert!( srcIdx < self._Ports.Size(), "Source port out of bounds");
        assert!( 
            dstIdx < self._Ports.Size(),
            "Destination port out of bounds"
        );
        let  	srcOwner = self._Ports[srcIdx].Owner();
        let  	dstOwner = self._Ports[dstIdx].Owner();
        let  	srcParent = self._Modules[srcOwner.Id()]._Parent;
        let  	dstParent = self._Modules[dstOwner.Id()]._Parent;
        let  	( driver, sink) = if srcParent == dstParent {
            // Sibling-to-Sibling
            assert!( 
                src.IsOut(),
                "In sibling connection, source must be an output port"
            );
            assert!( 
                dst.IsIn(),
                "In sibling connection, destination must be an input port"
            );
            ( src, dst)
        } else if srcOwner == dstParent {
            // Pass-Down: parent input driving child input
            assert!( 
                src.IsIn(),
                "In pass-down connection, parent port must be an input"
            );
            assert!( 
                dst.IsIn(),
                "In pass-down connection, child port must be an input"
            );
            ( src, dst)
        } else if dstOwner == srcParent {
            // Pass-Up: child output driving parent output
            assert!( 
                src.IsOut(),
                "In pass-up connection, child port must be an output"
            );
            assert!( 
                dst.IsOut(),
                "In pass-up connection, parent port must be an output"
            );
            ( src, dst)
        } else if srcOwner == dstOwner {
            // Feedthrough: parent input connected directly to parent output
            assert!( 
                src.IsIn(),
                "In feedthrough connection, source must be an input"
            );
            assert!( 
                dst.IsOut(),
                "In feedthrough connection, destination must be an output"
            );
            ( src, dst)
        } else {
            panic!( "Invalid hierarchy connection: port is not visible beyond immediate parent");
        };
        let  	srcType = self._Ports[srcIdx].Type();
        let  	dstType = self._Ports[dstIdx].Type();
        assert!( srcType == dstType, "Port type mismatch in connection");
        let  	ok = self._Netlist.Connect( driver, sink);
        assert!( ok, "Netlist connection failed: duplicate driver conflict");
        self
    }
    pub fn	SealModule( &mut self, moduleId: ModuleId)
    {
        let  	modIdx = moduleId.Id();
        assert!( modIdx < self._Modules.Size(), "ModuleId out of bounds");
        assert!( !self._Modules[modIdx]._IsSealed, "Module is already sealed");
        // 1. Gather all root IDs for boundary ports of this module
        let  	totalBoundary =
            self._Modules[modIdx]._InPorts.Size() + self._Modules[modIdx]._OutPorts.Size();
        let  	mut boundaryRoots = Stash::WithCapacity( totalBoundary);
        self._Modules[modIdx]._InPorts.Traverse( |idx| {
            boundaryRoots.Push( self._Netlist.FindRoot( PortId::In( idx)));
        });
        self._Modules[modIdx]._OutPorts.Traverse( |idx| {
            boundaryRoots.Push( self._Netlist.FindRoot( PortId::Out( idx)));
        });
        // 2. Traverse direct children of this module
        let  	childCount = self._ModuleChildren[modIdx].Size();
        USeg::FromLen( childCount).Traverse( |cIdx| {
            let  	childId = self._ModuleChildren[modIdx][cIdx];
            let  	child = &self._Modules[childId.Id()];
            assert!( child._IsSealed, "Child module must be sealed before parent");
            child._InPorts.Traverse( |idx| {
                let  	portId = PortId::In( idx);
                let  	root = self._Netlist.FindRoot( portId);
                let  	mut isBoundary = false;
                boundaryRoots.Arr().Traverse( |&r| {
                    if r == root {
                        isBoundary = true;
                    }
                });
                if !isBoundary && !self._Netlist.HasTrigger( portId) {
                    let  	pType = self._Ports[idx].Type();
                    self._Netlist.AssignTrigger( root, pType);
                }
            });
            child._OutPorts.Traverse( |idx| {
                let  	portId = PortId::Out( idx);
                let  	root = self._Netlist.FindRoot( portId);
                let  	mut isBoundary = false;
                boundaryRoots.Arr().Traverse( |&r| {
                    if r == root {
                        isBoundary = true;
                    }
                });
                if !isBoundary && !self._Netlist.HasTrigger( portId) {
                    let  	pType = self._Ports[idx].Type();
                    self._Netlist.AssignTrigger( root, pType);
                }
            });
        });
        // 3. If top-level module (parent is invalid), seal boundary ports too
        if !self._Modules[modIdx]._Parent.IsValid() {
            self._Modules[modIdx]._InPorts.Traverse( |idx| {
                let  	portId = PortId::In( idx);
                let  	root = self._Netlist.FindRoot( portId);
                if !self._Netlist.HasTrigger( portId) {
                    let  	pType = self._Ports[idx].Type();
                    self._Netlist.AssignTrigger( root, pType);
                }
            });
            self._Modules[modIdx]._OutPorts.Traverse( |idx| {
                let  	portId = PortId::Out( idx);
                let  	root = self._Netlist.FindRoot( portId);
                if !self._Netlist.HasTrigger( portId) {
                    let  	pType = self._Ports[idx].Type();
                    self._Netlist.AssignTrigger( root, pType);
                }
            });
        }
        self._Modules[modIdx]._IsSealed = true;
    }
    pub fn	SortModules( &mut self)
    {
        let  	modCount = self._Modules.Size();
        if modCount <= 1 {
            self._SubModules.Clear();
            self._Descendents.Clear();
            self._ModuleChildren.Clear();
            if modCount == 1 {
                self._Modules[0]._SubModules = USeg::Empty();
                self._Modules[0]._Descendents = USeg::Empty();
            }
            return;
        }
        // Sort modules by KernelKind ClassKey then Id
        let  	mut perm: Stash< u32> = Stash::WithCapacity( modCount);
        USeg::FromLen( modCount).Traverse( |i| {
            perm.Push( i);
        });
        perm.MutArr().QSort(
            |&mA, &mB| {
                let  	keyA = self._Modules[mA]._Kernel.ClassKey();
                let  	keyB = self._Modules[mB]._Kernel.ClassKey();
                if keyA == keyB {
                    self._Modules[mA]._Id.Id() < self._Modules[mB]._Id.Id()
                } else {
                    keyA < keyB
                }
            },
        );
        let  	mut sortedModules = Stash::WithCapacity( modCount);
        let  	mut oldToNew = Buff::FromDispenser( modCount, |_| ModuleId::Invalid());
        USeg::FromLen( modCount).Traverse( |newIdx| {
            let  	oldIdx = perm[newIdx];
            sortedModules.Push( self._Modules[oldIdx].clone());
            oldToNew[oldIdx] = ModuleId::New( newIdx);
        });
        self._Modules = sortedModules;
        self._SubModules.Clear();
        // Update module ids, port owners, and submodules
        USeg::FromLen( modCount).Traverse( |newIdx| {
            let  	oldId = self._Modules[newIdx]._Id;
            let  	newModId = ModuleId::New( newIdx);
            self._Modules[newIdx]._Id = newModId;
            self._Modules[newIdx]._InPorts.Traverse( |idx| {
                self._Ports[idx].SetOwner( newModId);
            });
            self._Modules[newIdx]._OutPorts.Traverse( |idx| {
                self._Ports[idx].SetOwner( newModId);
            });
            let  	start = self._SubModules.Size();
            let  	oldChildrenCount = self._ModuleChildren[oldId.Id()].Size();
            USeg::FromLen( oldChildrenCount).Traverse( |c| {
                let  	childModId = self._ModuleChildren[oldId.Id()][c];
                self._SubModules.Push( oldToNew[childModId.Id()]);
            });
            self._Modules[newIdx]._SubModules = USeg::WithLen( start, oldChildrenCount);
        });
        self._ModuleChildren.Clear();
    }
    pub fn	Freeze( &mut self)
    {
        let  	modCount = self._Modules.Size();
        USeg::FromLen( modCount).TraverseRev( |step| {
            let  	modId = ModuleId::New( step);
            if !self._Modules[modId.Id()]._IsSealed {
                self.SealModule( modId);
            }
        });
        self._PortToTrigger = self._Netlist.BuildPortToTrigger();
        self.SortModules();
    }
    pub fn	PortToTrigger( &self) -> Buff< TriggerId>
    {
        if self._PortToTrigger.Size() > 0 {
            return self._PortToTrigger.clone();
        }
        self._Netlist.BuildPortToTriggerConst()
    }
    pub fn	BuildTriggers( &self, portToTrigger: &Buff< TriggerId>) -> TriggerWad< u64>
    {
        let  	groupCount = self._Netlist.TriggerCount();
        let  	pastVals = Buff::FromDispenser( groupCount, |_| 0u64);
        let  	currentVals = Buff::FromDispenser( groupCount, |_| 0u64);
        let  	futureVals = Buff::FromDispenser( groupCount, |_| 0u64);
        let  	flags = Buff::FromDispenser( groupCount, |_| 0u8);
        let  	mut subscribersLists: Stash< Stash< u32>> = Stash::WithCapacity( groupCount);
        USeg::FromLen( groupCount).Traverse( |_| {
            subscribersLists.Push( Stash::New());
        });
        USeg::FromLen( self._Modules.Size()).Traverse( |m| {
            let  	module = &self._Modules[m];
            module._InPorts.Traverse( |portIdx| {
                let  	trigId = portToTrigger[portIdx];
                subscribersLists[trigId].Push( module._Id.Id());
            });
        });
        let  	mut subscriberSpans = Stash::WithCapacity( groupCount);
        let  	mut subscribers = Stash::New();
        USeg::FromLen( groupCount).Traverse( |g| {
            let  	start = subscribers.Size();
            subscribersLists[g].Arr().Traverse( |&sub| {
                subscribers.Push( sub);
            });
            subscriberSpans.Push( USeg::WithLen( start, subscribers.Size() - start));
        });
        TriggerWad::New( 
            pastVals,
            currentVals,
            futureVals,
            flags,
            subscriberSpans.ExtractBuff(),
            subscribers.ExtractBuff(),
        )
    }
    pub fn	CompileWarps( &self, portToTrigger: &Buff< TriggerId>) -> Buff< FastWarp>
    {
        let  	mut fastWarps = Stash::New();
        let  	mut i = 0u32;
        let  	modLen = self._Modules.Size();
        while i < modLen {
            let  	m = &self._Modules[i];
            let  	op = match m._Kernel.ToFastOp() {
                Some( op) => op,
                None => {
                    i += 1;
                    continue;
                }
            };
            let  	outPortIdx0 = m._OutPorts.First();
            let  	mask = self._Ports[outPortIdx0].Type().Mask();
            let  	startIdx = i;
            let  	mut in1List = Stash::New();
            let  	mut in2List = Stash::New();
            let  	mut outList = Stash::New();
            while i < modLen {
                let  	curMod = &self._Modules[i];
                if let  	Some( curOp) = curMod._Kernel.ToFastOp() {
                    let  	curOutPort0 = curMod._OutPorts.First();
                    let  	curMask = self._Ports[curOutPort0].Type().Mask();
                    if curOp == op && curMask == mask {
                        let  	in1 = portToTrigger[curMod._InPorts.First()];
                        let  	in2 = if curMod._InPorts.Size() > 1 {
                            portToTrigger[curMod._InPorts.First() + 1]
                        } else {
                            in1
                        };
                        let  	outTrig = portToTrigger[curOutPort0];
                        in1List.Push( in1);
                        in2List.Push( in2);
                        outList.Push( outTrig);
                        i += 1;
                        continue;
                    }
                }
                break;
            }
            let  	count = i - startIdx;
            fastWarps.Push( FastWarp::New( 
                op,
                startIdx,
                count,
                mask,
                in1List.ExtractBuff(),
                in2List.ExtractBuff(),
                outList.ExtractBuff(),
            ));
        }
        fastWarps.ExtractBuff()
    }
    pub fn	PortTriggersOf( &self, ports: USeg, portToTrigger: &Buff< TriggerId>) -> Buff< TriggerId>
    {
        let  	mut trigs = Stash::WithCapacity( ports.Size());
        ports.Traverse( |i| {
            trigs.Push( portToTrigger[i]);
        });
        trigs.ExtractBuff()
    }
    pub fn	CompileCoroWarps( &self, portToTrigger: &Buff< TriggerId>) -> Buff< CoroWarp>
    {
        let  	mut coroWarps = Stash::New();
        let  	mut i = 0u32;
        let  	modLen = self._Modules.Size();
        while i < modLen {
            let  	m = &self._Modules[i];
            if !m._Kernel.IsCoro() {
                i += 1;
                continue;
            }
            let  	key = m._Kernel.ClassKey();
            let  	startIdx = i;
            let  	mut instances = Stash::New();
            let  	mut inTriggersList = Stash::New();
            let  	mut outTriggersList = Stash::New();
            while i < modLen && self._Modules[i]._Kernel.ClassKey() == key {
                let  	curMod = &self._Modules[i];
                if let  	KernelKind::Coro( ref factory) = curMod._Kernel {
                    instances.Push( CoroCell::New( factory()));
                }
                inTriggersList.Push( self.PortTriggersOf( curMod._InPorts, portToTrigger));
                outTriggersList.Push( self.PortTriggersOf( curMod._OutPorts, portToTrigger));
                i += 1;
            }
            let  	count = i - startIdx;
            coroWarps.Push( CoroWarp::New( 
                startIdx,
                count,
                instances.ExtractBuff(),
                inTriggersList.ExtractBuff(),
                outTriggersList.ExtractBuff(),
            ));
        }
        coroWarps.ExtractBuff()
    }
}
