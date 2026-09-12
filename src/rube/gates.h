#ifndef TRELLIS_RUBE_GATES_H
#define TRELLIS_RUBE_GATES_H

//-------------------------------------------------------------------------------------------------

#include "rube/layout.h"
#include "rube/module.h"
#include "rube/port.h"

#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------

inline void CreateGate2(
    Layout& layout,
    const char* name,
    ModuleId parent,
    KernelOp op,
    ModuleId& outMod,
    PortId& outIn1,
    PortId& outIn2,
    PortId& outOut)
{
    PortDesc inDescs[2] = {PortDesc::Bool( "in1"), PortDesc::Bool( "in2")};
    PortDesc outDescs[1] = {PortDesc::Bool( "out")};

    outMod = layout.AddModule(
        name,
        parent,
        silo::Arr< const PortDesc>( inDescs, 2),
        silo::Arr< const PortDesc>( outDescs, 1),
        KernelKind::Fast( op)
    );
    outIn1 = layout.InPort( outMod, 0);
    outIn2 = layout.InPort( outMod, 1);
    outOut = layout.OutPort( outMod, 0);
    layout.SealModule( outMod);
}

//-------------------------------------------------------------------------------------------------

#define TRELLIS_DEFINE_GATE2( GateName, OpVal)                           \
class GateName                                                           \
{                                                                        \
    ModuleId    _Id{};                                                   \
    PortId      _In1{};                                                  \
    PortId      _In2{};                                                  \
    PortId      _Out{};                                                  \
public:                                                                  \
    constexpr GateName( void) noexcept = default;                        \
    GateName( Layout& layout, const char* name, ModuleId parent = ModuleId{}) \
    {                                                                    \
        CreateGate2( layout, name, parent, KernelOp::OpVal, _Id, _In1, _In2, _Out); \
    }                                                                    \
    constexpr ModuleId Id( void) const noexcept { return _Id; }          \
    constexpr PortId In1( void) const noexcept { return _In1; }          \
    constexpr PortId In2( void) const noexcept { return _In2; }          \
    constexpr PortId Out( void) const noexcept { return _Out; }          \
};

TRELLIS_DEFINE_GATE2( NandGate, Nand)
TRELLIS_DEFINE_GATE2( AndGate,  And)
TRELLIS_DEFINE_GATE2( OrGate,   Or)
TRELLIS_DEFINE_GATE2( XorGate,  Xor)
TRELLIS_DEFINE_GATE2( NorGate,  Nor)
TRELLIS_DEFINE_GATE2( XnorGate, Xnor)

#undef TRELLIS_DEFINE_GATE2

//-------------------------------------------------------------------------------------------------
// 1-Input Inverter / NOT Gate

class NotGate
{
    ModuleId    _Id{};
    PortId      _In{};
    PortId      _Out{};

public:
    constexpr NotGate( void) noexcept = default;

    NotGate( Layout& layout, const char* name, ModuleId parent = ModuleId{})
    {
        PortDesc inDescs[1] = {PortDesc::Bool( "in")};
        PortDesc outDescs[1] = {PortDesc::Bool( "out")};

        _Id = layout.AddModule(
            name,
            parent,
            silo::Arr< const PortDesc>( inDescs, 1),
            silo::Arr< const PortDesc>( outDescs, 1),
            KernelKind::Fast( KernelOp::Not)
        );
        _In = layout.InPort( _Id, 0);
        _Out = layout.OutPort( _Id, 0);
        layout.SealModule( _Id);
    }

    constexpr ModuleId Id( void) const noexcept { return _Id; }
    constexpr PortId In( void) const noexcept { return _In; }
    constexpr PortId Out( void) const noexcept { return _Out; }
};

} // namespace trellis::rube

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_RUBE_GATES_H

