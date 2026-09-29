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
    _Modules:           Stash< Module>,
    _Ports:             Stash< PortDesc>,
    _Netlist:           Netlist,
    _ModuleChildren:    Stash< Stash< ModuleId>>,
    _SubModules:        Stash< ModuleId>,
    _Descendents:       Stash< ModuleId>,
    _PortToTrigger:     Buff< TriggerId>,
    _HierarchyCompiled: bool,
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

    #[inline]
    pub fn  Modules( &self) -> Arr<'_, Module>
    {
        return self._Modules.Arr();
    }
    #[inline]
    pub fn  Module( &self, id: ModuleId) -> &Module
    {
        return &self._Modules[id.Id()];
    }
    #[inline]
    pub fn  Ports( &self) -> Arr<'_, PortDesc>
    {
        return self._Ports.Arr();
    }
    #[inline]
    pub fn  Port( &self, id: PortId) -> &PortDesc
    {
        return &self._Ports[id.Index()];
    }
    #[inline] pub fn Netlist(&self) -> &Netlist { &self._Netlist }

    pub fn	New() -> Self
    {
        Self {
            _Modules:           Stash::New(),
            _Ports:             Stash::New(),
            _Netlist:           Netlist::New(),
            _ModuleChildren:    Stash::New(),
            _SubModules:        Stash::New(),
            _Descendents:       Stash::New(),
            _PortToTrigger:     Buff::New(),
            _HierarchyCompiled: false,
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
        assert!( !self._HierarchyCompiled, "Cannot add modules after sorting or freezing");
        assert!( !parent.IsValid() || parent.Id() < self._Modules.Size(),
                 "Parent ModuleId must refer to an existing module");
        assert!( !parent.IsValid() || !self.Module( parent).IsSealed(),
                 "Cannot add children to a sealed module");
        let  	modId = ModuleId::New( self._Modules.Size());
        let  	inSeg = self.AddPorts( modId, name, inPorts);
        let  	outSeg = self.AddPorts( modId, name, outPorts);
        let  	module = Module::New( modId, parent, name, inSeg, outSeg, kernel);
        self._Modules.Push( module);
        self._ModuleChildren.Push( Stash::New());
        if parent.IsValid() {
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
        assert!( portIdx < module.InPorts().Size(), "Port index out of bounds");
        PortId::In( module.InPorts().First() + portIdx)
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
            portIdx < module.OutPorts().Size(),
            "Port index out of bounds"
        );
        PortId::Out( module.OutPorts().First() + portIdx)
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
        let  	srcParent = self._Modules[srcOwner.Id()].Parent();
        let  	dstParent = self._Modules[dstOwner.Id()].Parent();
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
        assert!( !self._Modules[modIdx].IsSealed(), "Module is already sealed");
        // 1. Gather all root IDs for boundary ports of this module
        let  	totalBoundary =
            self._Modules[modIdx].InPorts().Size() + self._Modules[modIdx].OutPorts().Size();
        let  	mut boundaryRoots = Stash::WithCapacity( totalBoundary);
        self._Modules[modIdx].InPorts().Traverse( |idx| {
            boundaryRoots.Push( self._Netlist.FindRoot( PortId::In( idx)));
        });
        self._Modules[modIdx].OutPorts().Traverse( |idx| {
            boundaryRoots.Push( self._Netlist.FindRoot( PortId::Out( idx)));
        });
        // 2. Traverse direct children of this module
        let childCount  = self.Children( moduleId).Size();
        USeg::FromLen( childCount).Traverse( |cIdx| {
            let childId     = self.Children( moduleId)[cIdx];
            let  	child = &self._Modules[childId.Id()];
            assert!( child.IsSealed(), "Child module must be sealed before parent");
            child.InPorts().Traverse( |idx| {
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
            child.OutPorts().Traverse( |idx| {
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
        if !self._Modules[modIdx].Parent().IsValid() {
            self._Modules[modIdx].InPorts().Traverse( |idx| {
                let  	portId = PortId::In( idx);
                let  	root = self._Netlist.FindRoot( portId);
                if !self._Netlist.HasTrigger( portId) {
                    let  	pType = self._Ports[idx].Type();
                    self._Netlist.AssignTrigger( root, pType);
                }
            });
            self._Modules[modIdx].OutPorts().Traverse( |idx| {
                let  	portId = PortId::Out( idx);
                let  	root = self._Netlist.FindRoot( portId);
                if !self._Netlist.HasTrigger( portId) {
                    let  	pType = self._Ports[idx].Type();
                    self._Netlist.AssignTrigger( root, pType);
                }
            });
        }
        self._Modules[modIdx].SetSealed();
    }
    /// Removes only the actual parent's qualified prefix, preserving independent names.
    pub fn  LocalName( &self, id: ModuleId) -> &str
    {
        let module      = self.Module( id);
        if module.Parent().IsValid()
        {
            let parent      = self.Module( module.Parent());
            if let  Some( suffix) = module.Name().strip_prefix( parent.Name())
                && let  Some( name) = suffix.strip_prefix( '.')
            {
                return name;
            }
        }
        return module.Name();
    }

    /// Borrows direct children before and after compilation. SortModules remaps ModuleIds.
    pub fn  Children( &self, id: ModuleId) -> Arr<'_, ModuleId>
    {
        let module      = self.Module( id);
        if !self._HierarchyCompiled
        {
            return self._ModuleChildren[id.Id()].Arr();
        }
        let span    = module.SubModules();
        return self._SubModules.Arr().Slice( span.First(), span.Size());
    }

    /// Borrows descendants in depth-first order after SortModules or Freeze, excluding self.
    pub fn  Descendants( &self, id: ModuleId) -> Arr<'_, ModuleId>
    {
        assert!( self._HierarchyCompiled, "Compile the hierarchy before querying descendants");
        let span    = self.Module( id).Descendents();
        return self._Descendents.Arr().Slice( span.First(), span.Size());
    }

    /// Walks all roots with balanced entry/exit events, including leaves; root depth is zero.
    /// False on entry prunes a subtree without an exit; false on exit stops the entire walk.
    /// The reusable stack grows with depth, independent of sibling count.
    pub fn  TraverseModules( &self, mut visit: impl FnMut( &Module, u32, bool) -> bool)
    {
        let mut stack   = Stash::New();
        let mut root    = 0;
        while root < self._Modules.Size()
        {
            if self._Modules[root].Parent().IsValid()
            {
                root += 1;
                continue;
            }
            stack.Push( ( ModuleId::New( root), u32::MAX));
            while let   Some( ( id, next)) = stack.Top()
            {
                let depth   = stack.Size() - 1;
                if next == u32::MAX
                {
                    if !visit( self.Module( id), depth, true)
                    {
                        stack.Pop();
                        continue;
                    }
                    stack[depth].1 = 0;
                }
                let next        = stack[depth].1;
                let children    = self.Children( id);
                if next < children.Size()
                {
                    stack[depth].1 += 1;
                    stack.Push( ( children[next], u32::MAX));
                }
                else
                {
                    if !visit( self.Module( id), depth, false)
                    {
                        return;
                    }
                    stack.Pop();
                }
            }
            root += 1;
        }
    }

    /// Groups kernels and compiles hierarchy indexes. Repeated calls are harmless.
    /// Callers must reacquire ModuleIds after sorting; PortIds remain stable.
    pub fn  SortModules( &mut self)
    {
        if self._HierarchyCompiled
        {
            return;
        }
        let modCount    = self._Modules.Size();
        // Sort modules by KernelKind ClassKey then Id
        let  	mut perm: Stash< u32> = Stash::WithCapacity( modCount);
        USeg::FromLen( modCount).Traverse( |i| {
            perm.Push( i);
        });
        perm.MutArr().QSort( 
            |&mA, &mB| {
                let  	keyA = self._Modules[mA].Kernel().ClassKey();
                let  	keyB = self._Modules[mB].Kernel().ClassKey();
                if keyA == keyB {
                    self._Modules[mA].Id().Id() < self._Modules[mB].Id().Id()
                } else {
                    keyA < keyB
                }
            },
        );
        let  	mut sortedModules = Stash::WithCapacity( modCount);
        let  	mut oldToNew = Buff::FromDispenser( modCount, |_| ModuleId::Invalid());
        USeg::FromLen( modCount).Traverse( |newIdx| {
            let  	oldIdx = perm[newIdx];
            sortedModules.Push( std::mem::take( &mut self._Modules[oldIdx]));
            oldToNew[oldIdx] = ModuleId::New( newIdx);
        });
        self._Modules = sortedModules;
        self._SubModules.Clear();
        // Update module ids, port owners, and submodules
        USeg::FromLen( modCount).Traverse( |newIdx| {
            let  	oldId = self._Modules[newIdx].Id();
            let  	newModId = ModuleId::New( newIdx);
            self._Modules[newIdx].SetId( newModId);
            let parent      = self._Modules[newIdx].Parent();
            if parent.IsValid()
            {
                self._Modules[newIdx].SetParent( oldToNew[parent.Id()]);
            }
            self._Modules[newIdx].InPorts().Traverse( |idx| {
                self._Ports[idx].SetOwner( newModId);
            });
            self._Modules[newIdx].OutPorts().Traverse( |idx| {
                self._Ports[idx].SetOwner( newModId);
            });
            let  	start = self._SubModules.Size();
            let  	oldChildrenCount = self._ModuleChildren[oldId.Id()].Size();
            USeg::FromLen( oldChildrenCount).Traverse( |c| {
                let  	childModId = self._ModuleChildren[oldId.Id()][c];
                self._SubModules.Push( oldToNew[childModId.Id()]);
            });
            self._Modules[newIdx].SetSubModules( USeg::WithLen( start, oldChildrenCount));
        });
        self._ModuleChildren = Stash::New();
        self._HierarchyCompiled = true;
        // One shared preorder stores every subtree as a range, using O(modules) space.
        let mut preorder    = Stash::WithCapacity( modCount);
        let mut spans       = Buff::FromDispenser( modCount, |_| USeg::Empty());
        self.TraverseModules( |module, _, enter| {
            let id      = module.Id().Id();
            if enter
            {
                preorder.Push( module.Id());
                // Preserve the start until exit; USeg::WithLen(_, 0) discards it.
                spans[id] = USeg::WithLen( preorder.Size(), 1);
            }
            else
            {
                let first   = spans[id].First();
                spans[id] = USeg::WithLen( first, preorder.Size() - first);
            }
            return true;
        });
        self._Descendents = preorder;
        USeg::FromLen( modCount).Traverse( |id| {
            self._Modules[id].SetDescendents( spans[id]);
        });
    }
    pub fn	Freeze( &mut self)
    {
        self.SortModules();
        // Reverse preorder seals children before parents without another traversal buffer.
        USeg::FromLen( self._Descendents.Size()).TraverseRev( |index| {
            let id      = self._Descendents[index];
            if !self.Module( id).IsSealed()
            {
                self.SealModule( id);
            }
        });
        self._PortToTrigger = self._Netlist.BuildPortToTrigger();
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
            module.InPorts().Traverse( |portIdx| {
                let  	trigId = portToTrigger[portIdx];
                subscribersLists[trigId].Push( module.Id().Id());
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
            let  	op = match m.Kernel().ToFastOp() {
                Some( op) => op,
                None => {
                    i += 1;
                    continue;
                }
            };
            let  	outPortIdx0 = m.OutPorts().First();
            let  	mask = self._Ports[outPortIdx0].Type().Mask();
            let  	startIdx = i;
            let  	mut in1List = Stash::New();
            let  	mut in2List = Stash::New();
            let  	mut outList = Stash::New();
            while i < modLen {
                let  	curMod = &self._Modules[i];
                if let  	Some( curOp) = curMod.Kernel().ToFastOp() {
                    let  	curOutPort0 = curMod.OutPorts().First();
                    let  	curMask = self._Ports[curOutPort0].Type().Mask();
                    if curOp == op && curMask == mask {
                        let  	in1 = portToTrigger[curMod.InPorts().First()];
                        let  	in2 = if curMod.InPorts().Size() > 1 {
                            portToTrigger[curMod.InPorts().First() + 1]
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
            if !m.Kernel().IsCoro() {
                i += 1;
                continue;
            }
            let  	key = m.Kernel().ClassKey();
            let  	startIdx = i;
            let  	mut instances = Stash::New();
            let  	mut inTriggersList = Stash::New();
            let  	mut outTriggersList = Stash::New();
            while i < modLen && self._Modules[i].Kernel().ClassKey() == key {
                let  	curMod = &self._Modules[i];
                if let  	KernelKind::Coro( factory) = curMod.Kernel() {
                    instances.Push( CoroCell::New( factory()));
                }
                inTriggersList.Push( self.PortTriggersOf( curMod.InPorts(), portToTrigger));
                outTriggersList.Push( self.PortTriggersOf( curMod.OutPorts(), portToTrigger));
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

//-------------------------------------------------------------------------------------------------
