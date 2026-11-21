// sta.rs ------------------------------------------------------------------------------------------------------
use	crate::rube::layout::Layout;
use	crate::rube::module::{ KernelKind, KernelOp };
use	crate::rube::port::{ ModuleId, PortId };
use	crate::rube::sdc::{ DelayKind, PathKind, SdcConstraints };
use	crate::rube::sdf::{ DelayCorner, SdfFile };
use	crate::silo::{ Buff, IArr, Stash, USeg };

//-------------------------------------------------------------------------------------------------

/// Represents a single node / pin in a timing path.
#[derive( Clone, Debug, PartialEq)]
pub struct StaPathNode
{
    _PortId:       PortId,
    _PortName:     String,
    _ArrivalTime:  f64,
    _RequiredTime: f64,
    _Slack:        f64,
    _DelayFromPre: f64,
}

impl StaPathNode
{
    pub fn	New(
        portId: PortId,
        portName: impl Into< String>,
        arr: f64,
        req: f64,
        slack: f64,
        delay: f64,
    ) -> Self
    {
        Self {
            _PortId:       portId,
            _PortName:     portName.into(),
            _ArrivalTime:  arr,
            _RequiredTime: req,
            _Slack:        slack,
            _DelayFromPre: delay,
        }
    }

    #[inline] pub const fn	Port( &self) -> PortId { self._PortId }
    #[inline] pub fn	PortName( &self) -> &str { &self._PortName }
    #[inline] pub const fn	ArrivalTime( &self) -> f64 { self._ArrivalTime }
    #[inline] pub const fn	RequiredTime( &self) -> f64 { self._RequiredTime }
    #[inline] pub const fn	Slack( &self) -> f64 { self._Slack }
    #[inline] pub const fn	Delay( &self) -> f64 { self._DelayFromPre }
}

//-------------------------------------------------------------------------------------------------

/// Represents an end-to-end timing path from a launch point to a capture point.
#[derive( Clone, Debug, PartialEq)]
pub struct StaTimingPath
{
    _StartPoint:   String,
    _EndPoint:     String,
    _Nodes:        Buff< StaPathNode>,
    _ArrivalTime:  f64,
    _RequiredTime: f64,
    _Slack:        f64,
    _IsMet:        bool,
}

impl StaTimingPath
{
    pub fn	New(
        startPoint: impl Into< String>,
        endPoint: impl Into< String>,
        nodes: Buff< StaPathNode>,
        arr: f64,
        req: f64,
        slack: f64,
    ) -> Self
    {
        let  	isMet = slack >= 0.0;
        Self {
            _StartPoint:   startPoint.into(),
            _EndPoint:     endPoint.into(),
            _Nodes:        nodes,
            _ArrivalTime:  arr,
            _RequiredTime: req,
            _Slack:        slack,
            _IsMet:        isMet,
        }
    }

    #[inline] pub fn	StartPoint( &self) -> &str { &self._StartPoint }
    #[inline] pub fn	EndPoint( &self) -> &str { &self._EndPoint }
    #[inline] pub fn	Nodes( &self) -> &Buff< StaPathNode> { &self._Nodes }
    #[inline] pub const fn	ArrivalTime( &self) -> f64 { self._ArrivalTime }
    #[inline] pub const fn	RequiredTime( &self) -> f64 { self._RequiredTime }
    #[inline] pub const fn	Slack( &self) -> f64 { self._Slack }
    #[inline] pub const fn	IsMet( &self) -> bool { self._IsMet }
}

//-------------------------------------------------------------------------------------------------

/// Complete Static Timing Analysis report.
#[derive( Clone, Debug, PartialEq)]
pub struct StaReport
{
    _Wns:         f64,
    _Tns:         f64,
    _ClockPeriod: f64,
    _Paths:       Buff< StaTimingPath>,
}

impl StaReport
{
    pub fn	New( wns: f64, tns: f64, clkPeriod: f64, paths: Buff< StaTimingPath>) -> Self
    {
        Self {
            _Wns:         wns,
            _Tns:         tns,
            _ClockPeriod: clkPeriod,
            _Paths:       paths,
        }
    }

    #[inline] pub const fn	Wns( &self) -> f64 { self._Wns }
    #[inline] pub const fn	Tns( &self) -> f64 { self._Tns }
    #[inline] pub const fn	ClockPeriod( &self) -> f64 { self._ClockPeriod }
    #[inline] pub fn	Paths( &self) -> &Buff< StaTimingPath> { &self._Paths }
    #[inline] pub const fn	IsMet( &self) -> bool { self._Wns >= 0.0 }

    pub fn	ViolationsCount( &self) -> u32
    {
        let  	mut count = 0u32;
        self._Paths.Arr().Traverse( |p| {
            if !p.IsMet()
            {
                count += 1;
            }
        });
        count
    }

    pub fn	CriticalPath( &self) -> Option< &StaTimingPath>
    {
        if self._Paths.Size() == 0
        {
            return None;
        }
        let  	mut worstIdx = 0usize;
        let  	mut worstSlack = f64::INFINITY;
        let  	paths = self._Paths.Arr();
        USeg::FromLen( paths.Len()).Traverse( |i| {
            let  	s = paths.Get( i).unwrap().Slack();
            if s < worstSlack
            {
                worstSlack = s;
                worstIdx = i as usize;
            }
        });
        paths.Get( worstIdx as u32)
    }

    pub fn	Summary( &self) -> String
    {
        let  	status = if self.IsMet() { "MET" } else { "VIOLATED" };
        format!(
            "STA Status: {}\nClock Period: {:.3} ns\nWNS: {:.3} ns\nTNS: {:.3} ns\nViolations: {}\nPaths Analyzed: {}",
            status,
            self._ClockPeriod,
            self._Wns,
            self._Tns,
            self.ViolationsCount(),
            self._Paths.Size()
        )
    }
}

//-------------------------------------------------------------------------------------------------

/// Standard cell gate delay lookup.
#[derive( Clone, Debug, PartialEq)]
pub struct GateDelayModel
{
    _WireDelay: f64,
    _NotDelay:  f64,
    _AndDelay:  f64,
    _OrDelay:   f64,
    _XorDelay:  f64,
    _AddDelay:  f64,
}

impl Default for GateDelayModel
{
    fn	default() -> Self
    {
        Self {
            _WireDelay: 0.05,
            _NotDelay:  0.25,
            _AndDelay:  0.40,
            _OrDelay:   0.45,
            _XorDelay:  0.75,
            _AddDelay:  1.20,
        }
    }
}

impl GateDelayModel
{
    pub const fn	New( wire: f64, notD: f64, andD: f64, orD: f64, xorD: f64, addD: f64) -> Self
    {
        Self {
            _WireDelay: wire,
            _NotDelay:  notD,
            _AndDelay:  andD,
            _OrDelay:   orD,
            _XorDelay:  xorD,
            _AddDelay:  addD,
        }
    }

    pub fn	CellDelay( &self, op: KernelOp) -> f64
    {
        match op
        {
            KernelOp::Not => self._NotDelay,
            KernelOp::And | KernelOp::Nand => self._AndDelay,
            KernelOp::Or | KernelOp::Nor => self._OrDelay,
            KernelOp::Xor | KernelOp::Xnor => self._XorDelay,
            KernelOp::Add | KernelOp::Sub => self._AddDelay,
            _ => self._AndDelay,
        }
    }

    #[inline] pub const fn	WireDelay( &self) -> f64 { self._WireDelay }
}

//-------------------------------------------------------------------------------------------------

struct TimingEdge
{
    _Src:   PortId,
    _Dst:   PortId,
    _Delay: f64,
}

//-------------------------------------------------------------------------------------------------

fn	CollectDescendantFastGates( layout: &Layout, modId: ModuleId, outGates: &mut Stash< ( ModuleId, KernelOp)>)
{
    let  	children = layout.Children( modId);
    let  	childCount = children.Size();
    USeg::FromLen( childCount).Traverse( |cIdx| {
        let  	childId = children[cIdx];
        let  	child = layout.Module( childId);
        if let KernelKind::Fast( op) = child.Kernel()
        {
            outGates.Push( ( childId, *op));
        }
        else
        {
            CollectDescendantFastGates( layout, childId, outGates);
        }
    });
}

//-------------------------------------------------------------------------------------------------

/// Static Timing Analysis Engine for Trellis EDA circuits.
pub struct StaEngine< 'a>
{
    _Layout:      &'a Layout,
    _TopMod:      ModuleId,
    _Constraints: &'a SdcConstraints,
    _DelayModel:  GateDelayModel,
    _Sdf:         Option< &'a SdfFile>,
    _SdfCorner:   DelayCorner,
}

impl< 'a> StaEngine< 'a>
{
    pub fn	New( layout: &'a Layout, topMod: ModuleId, sdc: &'a SdcConstraints) -> Self
    {
        Self {
            _Layout:      layout,
            _TopMod:      topMod,
            _Constraints: sdc,
            _DelayModel:  GateDelayModel::default(),
            _Sdf:         None,
            _SdfCorner:   DelayCorner::Typ,
        }
    }

    pub fn	WithDelayModel( mut self, model: GateDelayModel) -> Self
    {
        self._DelayModel = model;
        self
    }

    pub fn	WithSdf( mut self, sdf: &'a SdfFile, corner: DelayCorner) -> Self
    {
        self._Sdf = Some( sdf);
        self._SdfCorner = corner;
        self
    }

    /// Executes Static Timing Analysis on the layout module hierarchy.
    pub fn	Run( &self) -> StaReport
    {
        let  	topModule = self._Layout.Module( self._TopMod);
        let  	totalPorts = self._Layout.Ports().Size() as usize;

        // Resolve clock period
        let  	mut clkPeriod = 10.0f64;
        self._Constraints.Clocks().Arr().Traverse( |clk| {
            if clk.Period() > 0.0
            {
                clkPeriod = clk.Period();
            }
        });

        // 1. Build timing graph edges
        let  	mut edges: Stash< TimingEdge> = Stash::New();

        // 1a. Interconnect edges from layout.Connections
        self._Layout.Connections().Traverse( |&( drv, sink)| {
            let  	mut delay = self._DelayModel.WireDelay();
            if let Some( sdf ) = self._Sdf
            {
                let  	drvFull = self._Layout.Port( drv).Name();
                let  	sinkFull = self._Layout.Port( sink).Name();
                let  	drvLocal = self._Layout.LocalPortName( drv);
                let  	sinkLocal = self._Layout.LocalPortName( sink);
                let  	topName = self._Layout.Module( self._TopMod).Name();
                let  	topPrefix = format!( "{}.", topName);
                let  	drvRel = drvFull.strip_prefix( &topPrefix).unwrap_or( drvFull);
                let  	sinkRel = sinkFull.strip_prefix( &topPrefix).unwrap_or( sinkFull);

                sdf.Cells().Arr().Traverse( |cell| {
                    cell.Interconnects().Arr().Traverse( |ic| {
                        let  	startMatch = ic.StartPort() == drvFull
                            || ic.StartPort() == drvRel
                            || ic.StartPort() == drvLocal
                            || ic.StartPort().replace( '/', ".") == drvRel
                            || ic.StartPort().replace( '/', ".") == drvFull;
                        let  	endMatch = ic.EndPort() == sinkFull
                            || ic.EndPort() == sinkRel
                            || ic.EndPort() == sinkLocal
                            || ic.EndPort().replace( '/', ".") == sinkRel
                            || ic.EndPort().replace( '/', ".") == sinkFull;
                        if startMatch && endMatch
                        {
                            delay = ic.Delay().Select( self._SdfCorner);
                        }
                    });
                });
            }
            edges.Push( TimingEdge {
                _Src:   drv,
                _Dst:   sink,
                _Delay: delay,
            });
        });

        // 1b. Internal gate propagation edges (across all hierarchical descendants)
        let  	mut allGates: Stash< ( ModuleId, KernelOp)> = Stash::New();
        CollectDescendantFastGates( self._Layout, self._TopMod, &mut allGates);

        allGates.Arr().Traverse( |&( childId, op)| {
            let  	child = self._Layout.Module( childId);
            let  	baseDelay = self._DelayModel.CellDelay( op);
            let  	outPort = self._Layout.OutPort( childId, 0);
            let  	outLocal = self._Layout.LocalPortName( outPort);
            let  	instLocal = self._Layout.LocalName( childId);
            let  	instFull = child.Name();
            let  	topName = self._Layout.Module( self._TopMod).Name();
            let  	topPrefix = format!( "{}.", topName);
            let  	instRel = instFull.strip_prefix( &topPrefix).unwrap_or( instFull);
            let  	inCount = child.InPorts().Size();

            USeg::FromLen( inCount).Traverse( |i| {
                let  	inPort = self._Layout.InPort( childId, i);
                let  	inLocal = self._Layout.LocalPortName( inPort);
                let  	mut delay = baseDelay;

                if let Some( sdf ) = self._Sdf
                {
                    sdf.Cells().Arr().Traverse( |cell| {
                        let  	cellInst = cell.Instance();
                        let  	instMatch = cellInst == instLocal
                            || cellInst == instRel
                            || cellInst == instFull
                            || cellInst.replace( '/', ".") == instRel
                            || cellInst.replace( '/', ".") == instFull
                            || cellInst == "*"
                            || cellInst.is_empty();
                        if instMatch
                        {
                            cell.IoPaths().Arr().Traverse( |iopath| {
                                if iopath.InputPort() == inLocal && iopath.OutputPort() == outLocal
                                {
                                    delay = iopath.AverageDelay( self._SdfCorner);
                                }
                            });
                        }
                    });
                }

                edges.Push( TimingEdge {
                    _Src:   inPort,
                    _Dst:   outPort,
                    _Delay: delay,
                });
            });
        });

        // 2. Forward Propagation: Arrival Times (T_arr)
        // Array indexed by PortId::Index()
        let  	mut arrTimes = vec![0.0f64; totalPorts];
        let  	mut predPort = vec![PortId::Invalid(); totalPorts];
        let  	mut predDelay = vec![0.0f64; totalPorts];

        // Seed primary inputs from SDC set_input_delay constraints
        let  	inCount = topModule.InPorts().Size();
        USeg::FromLen( inCount).Traverse( |i| {
            let  	inPort = self._Layout.InPort( self._TopMod, i);
            let  	name = self._Layout.LocalPortName( inPort);
            let  	mut seedArr = 0.0f64;
            self._Constraints.Delays().Arr().Traverse( |d| {
                if d.Kind() == DelayKind::Input && d.PortName() == name
                {
                    seedArr = d.Delay();
                }
            });
            arrTimes[inPort.Index() as usize] = seedArr;
        });

        // Propagate in relaxation passes (DAG diameter <= 64)
        let  	edgeArr = edges.Arr();
        let  	edgeCount = edgeArr.Len();
        let  	passes = 64u32;
        USeg::FromLen( passes).Span( |_| {
            let  	mut changed = false;
            USeg::FromLen( edgeCount).Traverse( |eIdx| {
                let  	edge = edgeArr.Get( eIdx).unwrap();
                let  	u = edge._Src.Index() as usize;
                let  	v = edge._Dst.Index() as usize;
                let  	candArr = arrTimes[u] + edge._Delay;
                if candArr > arrTimes[v] + 1e-9
                {
                    arrTimes[v] = candArr;
                    predPort[v] = edge._Src;
                    predDelay[v] = edge._Delay;
                    changed = true;
                }
            });
            changed
        });

        // 3. Backward Propagation: Required Times (T_req)
        let  	mut reqTimes = vec![f64::INFINITY; totalPorts];

        // Seed primary outputs from SDC set_output_delay constraints
        let  	outCount = topModule.OutPorts().Size();
        USeg::FromLen( outCount).Traverse( |i| {
            let  	outPort = self._Layout.OutPort( self._TopMod, i);
            let  	name = self._Layout.LocalPortName( outPort);
            let  	mut outDelay = 0.0f64;
            self._Constraints.Delays().Arr().Traverse( |d| {
                if d.Kind() == DelayKind::Output && d.PortName() == name
                {
                    outDelay = d.Delay();
                }
            });

            // Check for multicycle path exception targeting this port
            let  	mut cycles = 1u32;
            self._Constraints.Paths().Arr().Traverse( |p| {
                if p.Kind() == PathKind::MulticyclePath
                    && let Some( toPort ) = p.To()
                    && toPort == name
                {
                    cycles = p.Cycles();
                }
            });

            let  	targetReq = ( cycles as f64) * clkPeriod - outDelay;
            reqTimes[outPort.Index() as usize] = targetReq;
        });

        // Backward relaxation passes
        USeg::FromLen( passes).Span( |_| {
            let  	mut changed = false;
            USeg::FromLen( edgeCount).Traverse( |eIdx| {
                let  	edge = edgeArr.Get( eIdx).unwrap();
                let  	u = edge._Src.Index() as usize;
                let  	v = edge._Dst.Index() as usize;
                if reqTimes[v] < f64::INFINITY
                {
                    let  	candReq = reqTimes[v] - edge._Delay;
                    if candReq < reqTimes[u] - 1e-9
                    {
                        reqTimes[u] = candReq;
                        changed = true;
                    }
                }
            });
            changed
        });

        // 4. Construct Timing Paths for all endpoints (primary outputs)
        let  	mut paths: Stash< StaTimingPath> = Stash::New();
        let  	mut wns = f64::INFINITY;
        let  	mut tns = 0.0f64;

        USeg::FromLen( outCount).Traverse( |i| {
            let  	outPort = self._Layout.OutPort( self._TopMod, i);
            let  	endName = self._Layout.LocalPortName( outPort).to_string();
            let  	endIdx = outPort.Index() as usize;
            let  	arr = arrTimes[endIdx];
            let  	req = reqTimes[endIdx];

            // Check if false path exception applies
            let  	mut isFalsePath = false;
            self._Constraints.Paths().Arr().Traverse( |p| {
                if p.Kind() == PathKind::FalsePath
                    && let Some( toPort ) = p.To()
                    && toPort == endName
                {
                    isFalsePath = true;
                }
            });

            if isFalsePath
            {
                return;
            }

            let  	slack = req - arr;
            if slack < wns
            {
                wns = slack;
            }
            if slack < 0.0
            {
                tns += slack;
            }

            // Backtrace critical path to launch point
            let  	mut pathNodes: Vec< StaPathNode> = Vec::new();
            let  	mut curr = outPort;
            while curr.IsValid()
            {
                let  	idx = curr.Index() as usize;
                let  	pName = self._Layout.LocalPortName( curr).to_string();
                let  	pArr = arrTimes[idx];
                let  	pReq = reqTimes[idx];
                let  	pSlack = pReq - pArr;
                let  	pDel = predDelay[idx];
                pathNodes.push( StaPathNode::New( curr, pName, pArr, pReq, pSlack, pDel));
                curr = predPort[idx];
            }
            pathNodes.reverse();

            let  	startName = if !pathNodes.is_empty()
            {
                pathNodes[0].PortName().to_string()
            }
            else
            {
                "unknown".to_string()
            };

            let  	mut nodeStash = Stash::WithCapacity( pathNodes.len() as u32);
            USeg::FromLen( pathNodes.len() as u32).Traverse( |k| {
                nodeStash.Push( pathNodes[k as usize].clone());
            });

            paths.Push( StaTimingPath::New( startName, endName, nodeStash.IntoBuff(), arr, req, slack));
        });

        if wns == f64::INFINITY
        {
            wns = 0.0;
        }

        StaReport::New( wns, tns, clkPeriod, paths.IntoBuff())
    }
}

//-------------------------------------------------------------------------------------------------

/// Helper function to perform STA analysis on a layout with SDC constraints.
pub fn	RunSta( layout: &Layout, topMod: ModuleId, sdc: &SdcConstraints) -> StaReport
{
    StaEngine::New( layout, topMod, sdc).Run()
}

/// Helper function to perform STA analysis on a layout with SDC constraints and SDF back-annotation.
pub fn	RunStaWithSdf(
    layout: &Layout,
    topMod: ModuleId,
    sdc: &SdcConstraints,
    sdf: &SdfFile,
    corner: DelayCorner,
) -> StaReport
{
    StaEngine::New( layout, topMod, sdc).WithSdf( sdf, corner).Run()
}
