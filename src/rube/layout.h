// layout.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "rube/module.h"
#include "rube/netlist.h"
#include "rube/port.h"
#include "rube/trigger.h"
#include "silo/buff.h"
#include "silo/stash.h"

#include <algorithm>
#include <cassert>
#include <cstdint>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------
// Layout — manages module hierarchy, netlist connections, freezing, and warp compilation.
// Modeled directly from Kosh rube/layout.rs.

class Layout
{
public:
    silo::Stash< Module>                    _Modules{};
    silo::Stash< PortDesc>                  _Ports{};
    Netlist                                 _Netlist{};
    silo::Stash< silo::Stash< ModuleId>>    _ModuleChildren{};
    silo::Stash< ModuleId>                  _SubModules{};
    silo::Stash< ModuleId>                  _Descendents{};
    silo::Buff< TriggerId>                  _PortToTrigger{};

public:
    constexpr Layout( void) noexcept = default;

    silo::USeg AddPorts(
        ModuleId modId,
        const char* moduleName,
        silo::Arr< const PortDesc> ports)
    {
        const uint32_t start = _Ports.Size();
        const uint32_t count = ports.Size();
        for ( uint32_t i = 0; i < count; ++i) {
            PortDesc desc = ports[i];
            desc._Name = std::string( moduleName) + "." + desc._Name;
            desc._Owner = modId;
            _Ports.PushBack( desc);
        }
        _Netlist.Grow( count);
        return silo::USeg( start, count);
    }

    ModuleId AddModule(
        const char* name,
        ModuleId parent,
        silo::Arr< const PortDesc> inPorts,
        silo::Arr< const PortDesc> outPorts,
        KernelKind kernel)
    {
        const ModuleId  modId{_Modules.Size()};
        const silo::USeg inSeg = AddPorts( modId, name, inPorts);
        const silo::USeg outSeg = AddPorts( modId, name, outPorts);

        Module module( modId, parent, name, inSeg, outSeg, kernel);
        _Modules.PushBack( module);
        _ModuleChildren.PushBack( silo::Stash< ModuleId>{});

        if ( parent.IsValid()) {
            assert( parent._Id < _Modules.Size() && "Parent ModuleId out of bounds");
            _ModuleChildren[parent._Id].PushBack( modId);
        }

        return modId;
    }

    ModuleId AddCoroModule(
        const char* name,
        ModuleId parent,
        silo::Arr< const PortDesc> inPorts,
        silo::Arr< const PortDesc> outPorts,
        CoroKernelFactory factory)
    {
        return AddModule( name, parent, inPorts, outPorts, KernelKind::Coro( std::move( factory)));
    }

    PortId InPort( ModuleId moduleId, uint32_t portIdx) const
    {
        assert( moduleId._Id < _Modules.Size() && "ModuleId out of bounds");
        const Module& module = _Modules[moduleId._Id];
        assert( portIdx < module._InPorts.Size() && "Port index out of bounds");
        return PortId::In( module._InPorts.First() + portIdx);
    }

    PortId OutPort( ModuleId moduleId, uint32_t portIdx) const
    {
        assert( moduleId._Id < _Modules.Size() && "ModuleId out of bounds");
        const Module& module = _Modules[moduleId._Id];
        assert( portIdx < module._OutPorts.Size() && "Port index out of bounds");
        return PortId::Out( module._OutPorts.First() + portIdx);
    }

    Layout& Connect( PortId src, PortId dst)
    {
        const uint32_t srcIdx = src.Index();
        const uint32_t dstIdx = dst.Index();
        assert( srcIdx < _Ports.Size() && "Source port out of bounds");
        assert( dstIdx < _Ports.Size() && "Destination port out of bounds");

        const ModuleId srcOwner = _Ports[srcIdx]._Owner;
        const ModuleId dstOwner = _Ports[dstIdx]._Owner;
        const ModuleId srcParent = _Modules[srcOwner._Id]._Parent;
        const ModuleId dstParent = _Modules[dstOwner._Id]._Parent;

        PortId driver;
        PortId sink;

        if ( srcParent == dstParent) {
            // Sibling-to-Sibling
            assert( src.IsOut() && "In sibling connection, source must be an output port");
            assert( dst.IsIn() && "In sibling connection, destination must be an input port");
            driver = src;
            sink = dst;
        } else if ( srcOwner == dstParent) {
            // Pass-Down: parent input driving child input
            assert( src.IsIn() && "In pass-down connection, parent port must be an input");
            assert( dst.IsIn() && "In pass-down connection, child port must be an input");
            driver = src;
            sink = dst;
        } else if ( dstOwner == srcParent) {
            // Pass-Up: child output driving parent output
            assert( src.IsOut() && "In pass-up connection, child port must be an output");
            assert( dst.IsOut() && "In pass-up connection, parent port must be an output");
            driver = src;
            sink = dst;
        } else if ( srcOwner == dstOwner) {
            // Feedthrough: parent input connected directly to parent output
            assert( src.IsIn() && "In feedthrough connection, source must be an input");
            assert( dst.IsOut() && "In feedthrough connection, destination must be an output");
            driver = src;
            sink = dst;
        } else {
            assert( false && "Invalid hierarchy connection: port is not visible beyond immediate parent");
        }

        const PortType srcType = _Ports[srcIdx]._Type;
        const PortType dstType = _Ports[dstIdx]._Type;
        assert( srcType == dstType && "Port type mismatch in connection");

        bool ok = _Netlist.Connect( driver, sink);
        assert( ok && "Netlist connection failed");

        return *this;
    }

    void SealModule( ModuleId moduleId)
    {
        const uint32_t modIdx = moduleId._Id;
        assert( modIdx < _Modules.Size() && "ModuleId out of bounds");
        assert( !_Modules[modIdx]._IsSealed && "Module is already sealed");

        // 1. Gather all root IDs for boundary ports of this module
        const uint32_t totalBoundary = _Modules[modIdx]._InPorts.Size() + _Modules[modIdx]._OutPorts.Size();
        silo::Stash< uint32_t> boundaryRoots( totalBoundary, 0, static_cast< uint32_t>( 0));
        for ( uint32_t i = 0; i < _Modules[modIdx]._InPorts.Size(); ++i) {
            const uint32_t idx = _Modules[modIdx]._InPorts.First() + i;
            boundaryRoots.PushBack( _Netlist.FindRoot( PortId::In( idx)));
        }
        for ( uint32_t i = 0; i < _Modules[modIdx]._OutPorts.Size(); ++i) {
            const uint32_t idx = _Modules[modIdx]._OutPorts.First() + i;
            boundaryRoots.PushBack( _Netlist.FindRoot( PortId::Out( idx)));
        }
        std::sort( boundaryRoots.begin(), boundaryRoots.end());

        // 2. Traverse direct children of this module
        const uint32_t childCount = _ModuleChildren[modIdx].Size();
        for ( uint32_t cIdx = 0; cIdx < childCount; ++cIdx) {
            const ModuleId childId = _ModuleChildren[modIdx][cIdx];
            const Module& child = _Modules[childId._Id];
            assert( child._IsSealed && "Child module must be sealed before parent");

            for ( uint32_t i = 0; i < child._InPorts.Size(); ++i) {
                const uint32_t idx = child._InPorts.First() + i;
                const PortId portId = PortId::In( idx);
                const uint32_t root = _Netlist.FindRoot( portId);
                const bool isBoundary = std::binary_search( boundaryRoots.begin(), boundaryRoots.end(), root);
                if ( !isBoundary && !_Netlist.HasTrigger( portId)) {
                    _Netlist.AssignTrigger( root, _Ports[idx]._Type);
                }
            }

            for ( uint32_t i = 0; i < child._OutPorts.Size(); ++i) {
                const uint32_t idx = child._OutPorts.First() + i;
                const PortId portId = PortId::Out( idx);
                const uint32_t root = _Netlist.FindRoot( portId);
                const bool isBoundary = std::binary_search( boundaryRoots.begin(), boundaryRoots.end(), root);
                if ( !isBoundary && !_Netlist.HasTrigger( portId)) {
                    _Netlist.AssignTrigger( root, _Ports[idx]._Type);
                }
            }
        }

        // 3. If top-level module (parent is invalid), seal boundary ports too
        if ( !_Modules[modIdx]._Parent.IsValid()) {
            for ( uint32_t i = 0; i < _Modules[modIdx]._InPorts.Size(); ++i) {
                const uint32_t idx = _Modules[modIdx]._InPorts.First() + i;
                const PortId portId = PortId::In( idx);
                const uint32_t root = _Netlist.FindRoot( portId);
                if ( !_Netlist.HasTrigger( portId)) {
                    _Netlist.AssignTrigger( root, _Ports[idx]._Type);
                }
            }

            for ( uint32_t i = 0; i < _Modules[modIdx]._OutPorts.Size(); ++i) {
                const uint32_t idx = _Modules[modIdx]._OutPorts.First() + i;
                const PortId portId = PortId::Out( idx);
                const uint32_t root = _Netlist.FindRoot( portId);
                if ( !_Netlist.HasTrigger( portId)) {
                    _Netlist.AssignTrigger( root, _Ports[idx]._Type);
                }
            }
        }

        _Modules[modIdx]._IsSealed = true;
    }

    void SortModules( void)
    {
        const uint32_t modCount = _Modules.Size();
        if ( modCount <= 1) {
            _SubModules.Clear();
            _Descendents.Clear();
            _ModuleChildren.Clear();
            if ( modCount == 1) {
                _Modules[0]._SubModules = silo::USeg( 0, 0);
                _Modules[0]._Descendents = silo::USeg( 0, 0);
            }
            return;
        }

        // Sort modules by KernelKind ClassKey then Id
        silo::Buff< uint32_t> perm( modCount, []( uint32_t i) {
            return i;
        });

        std::sort( perm.begin(), perm.end(), [&]( uint32_t a, uint32_t b) {
            const auto keyA = _Modules[a]._Kernel.ClassKey();
            const auto keyB = _Modules[b]._Kernel.ClassKey();
            if ( keyA == keyB) {
                return _Modules[a]._Id < _Modules[b]._Id;
            }
            return keyA < keyB;
        });

        silo::Stash< Module> sortedModules( modCount, 0, Module{});
        silo::Buff< ModuleId> oldToNew( modCount, ModuleId{});

        for ( uint32_t newIdx = 0; newIdx < modCount; ++newIdx) {
            const uint32_t oldIdx = perm[newIdx];
            sortedModules.PushBack( _Modules[oldIdx]);
            oldToNew[oldIdx] = ModuleId{newIdx};
        }

        _Modules = std::move( sortedModules);
        _SubModules.Clear();

        // Update module ids, port owners, and submodules
        for ( uint32_t newIdx = 0; newIdx < modCount; ++newIdx) {
            const ModuleId oldId = _Modules[newIdx]._Id;
            const ModuleId newModId{newIdx};
            _Modules[newIdx]._Id = newModId;

            for ( uint32_t i = 0; i < _Modules[newIdx]._InPorts.Size(); ++i) {
                const uint32_t idx = _Modules[newIdx]._InPorts.First() + i;
                _Ports[idx]._Owner = newModId;
            }
            for ( uint32_t i = 0; i < _Modules[newIdx]._OutPorts.Size(); ++i) {
                const uint32_t idx = _Modules[newIdx]._OutPorts.First() + i;
                _Ports[idx]._Owner = newModId;
            }

            const uint32_t start = _SubModules.Size();
            const auto& oldChildren = _ModuleChildren[oldId._Id];
            for ( uint32_t c = 0; c < oldChildren.Size(); ++c) {
                _SubModules.PushBack( oldToNew[oldChildren[c]._Id]);
            }
            _Modules[newIdx]._SubModules = silo::USeg( start, oldChildren.Size());
        }

        _ModuleChildren.Clear();
    }

    void Freeze( void)
    {
        const uint32_t modCount = _Modules.Size();
        for ( uint32_t step = 0; step < modCount; ++step) {
            const ModuleId modId{modCount - 1 - step};
            if ( !_Modules[modId._Id]._IsSealed) {
                SealModule( modId);
            }
        }

        _PortToTrigger = _Netlist.BuildPortToTrigger();
        SortModules();
    }

    silo::Buff< TriggerId> PortToTrigger( void) const
    {
        if ( _PortToTrigger.Size() > 0) {
            return _PortToTrigger;
        }
        return _Netlist.BuildPortToTriggerConst();
    }

    TriggerWad< uint64_t> BuildTriggers( const silo::Buff< TriggerId>& portToTrigger) const
    {
        const uint32_t groupCount = _Netlist.TriggerCount();
        silo::Buff< uint64_t> pastVals( groupCount, static_cast< uint64_t>( 0));
        silo::Buff< uint64_t> currentVals( groupCount, static_cast< uint64_t>( 0));
        silo::Buff< uint64_t> futureVals( groupCount, static_cast< uint64_t>( 0));
        silo::Buff< uint8_t> flags( groupCount, static_cast< uint8_t>( 0));

        silo::Buff< silo::Stash< uint32_t>> subscribersLists( groupCount, []( uint32_t) {
            return silo::Stash< uint32_t>{};
        });
        for ( uint32_t m = 0; m < _Modules.Size(); ++m) {
            const Module& module = _Modules[m];
            for ( uint32_t i = 0; i < module._InPorts.Size(); ++i) {
                const uint32_t portIdx = module._InPorts.First() + i;
                const TriggerId trigId = portToTrigger[portIdx];
                subscribersLists[trigId].PushBack( module._Id._Id);
            }
        }

        silo::Stash< silo::USeg> subscriberSpans;
        silo::Stash< uint32_t> subscribers;

        for ( uint32_t g = 0; g < groupCount; ++g) {
            const uint32_t start = subscribers.Size();
            for ( uint32_t sub : subscribersLists[g]) {
                subscribers.PushBack( sub);
            }
            subscriberSpans.PushBack( silo::USeg( start, subscribers.Size() - start));
        }

        return TriggerWad< uint64_t>(
            std::move( pastVals),
            std::move( currentVals),
            std::move( futureVals),
            std::move( flags),
            subscriberSpans.ExtractBuff(),
            subscribers.ExtractBuff()
        );
    }

    silo::Buff< FastWarp> CompileWarps( const silo::Buff< TriggerId>& portToTrigger) const
    {
        silo::Stash< FastWarp> fastWarps;

        uint32_t i = 0;
        const uint32_t modLen = _Modules.Size();

        while ( i < modLen) {
            const Module& m = _Modules[i];
            if ( !m._Kernel.ToFastOp().has_value()) {
                ++i;
                continue;
            }

            const KernelOp op = *m._Kernel.ToFastOp();
            const uint32_t outPortIdx0 = m._OutPorts.First();
            const uint64_t mask = _Ports[outPortIdx0]._Type.Mask();
            const uint32_t startIdx = i;

            silo::Stash< TriggerId> in1List;
            silo::Stash< TriggerId> in2List;
            silo::Stash< TriggerId> outList;

            while ( i < modLen) {
                const Module& curMod = _Modules[i];
                if ( auto curOp = curMod._Kernel.ToFastOp()) {
                    const uint32_t curOutPort0 = curMod._OutPorts.First();
                    const uint64_t curMask = _Ports[curOutPort0]._Type.Mask();
                    if ( *curOp == op && curMask == mask) {
                        const TriggerId in1 = portToTrigger[curMod._InPorts.First()];
                        const TriggerId in2 = ( curMod._InPorts.Size() > 1)
                            ? portToTrigger[curMod._InPorts.First() + 1]
                            : in1;
                        const TriggerId outTrig = portToTrigger[curOutPort0];

                        in1List.PushBack( in1);
                        in2List.PushBack( in2);
                        outList.PushBack( outTrig);
                        ++i;
                        continue;
                    }
                }
                break;
            }

            const uint32_t count = i - startIdx;
            fastWarps.PushBack( FastWarp(
                op,
                startIdx,
                count,
                mask,
                in1List.ExtractBuff(),
                in2List.ExtractBuff(),
                outList.ExtractBuff()
            ));
        }

        return fastWarps.ExtractBuff();
    }

    silo::Buff< TriggerId> PortTriggersOf( silo::USeg ports, const silo::Buff< TriggerId>& portToTrigger) const
    {
        silo::Stash< TriggerId> trigs;
        for ( uint32_t i = 0; i < ports.Size(); ++i) {
            trigs.PushBack( portToTrigger[ports.First() + i]);
        }
        return trigs.ExtractBuff();
    }

    silo::Buff< CoroWarp> CompileCoroWarps( const silo::Buff< TriggerId>& portToTrigger) const
    {
        silo::Stash< CoroWarp> coroWarps;
        uint32_t i = 0;
        const uint32_t modLen = _Modules.Size();

        while ( i < modLen) {
            const Module& m = _Modules[i];
            if ( !m._Kernel.IsCoro()) {
                ++i;
                continue;
            }

            const auto key = m._Kernel.ClassKey();
            const uint32_t startIdx = i;
            silo::Stash< CoroCell> instances;
            silo::Stash< silo::Buff< TriggerId>> inTriggersList;
            silo::Stash< silo::Buff< TriggerId>> outTriggersList;

            while ( i < modLen && _Modules[i]._Kernel.ClassKey() == key) {
                const Module& curMod = _Modules[i];
                if ( curMod._Kernel.IsCoro()) {
                    const auto& factory = curMod._Kernel.ToCoroFactory();
                    instances.PushBack( CoroCell( factory()));
                }
                inTriggersList.PushBack( PortTriggersOf( curMod._InPorts, portToTrigger));
                outTriggersList.PushBack( PortTriggersOf( curMod._OutPorts, portToTrigger));
                ++i;
            }

            const uint32_t count = i - startIdx;
            coroWarps.PushBack( CoroWarp(
                startIdx,
                count,
                instances.ExtractBuff(),
                inTriggersList.ExtractBuff(),
                outTriggersList.ExtractBuff()
            ));
        }

        return coroWarps.ExtractBuff();
    }
};

} // namespace trellis::rube
