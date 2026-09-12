#ifndef TRELLIS_RUBE_PORT_H
#define TRELLIS_RUBE_PORT_H

//-------------------------------------------------------------------------------------------------

#include <cstdint>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------
// ModuleId — 32-bit module identifier.

struct ModuleId
{
    uint32_t    _Id{0xFFFF'FFFF};

    constexpr ModuleId( void) noexcept = default;
    constexpr explicit ModuleId( uint32_t id) noexcept : _Id( id) {}

    constexpr bool operator==( const ModuleId& other) const noexcept = default;
    constexpr bool operator<( const ModuleId& other) const noexcept { return _Id < other._Id; }
    constexpr bool IsValid( void) const noexcept { return _Id != 0xFFFF'FFFF; }
};

//-------------------------------------------------------------------------------------------------
// Direction of a port.

enum class PortDir
{
    In,
    Out,
};

//-------------------------------------------------------------------------------------------------
// PortId — 32-bit port index with direction encoded in bit 31.
// Modeled directly from Kosh rube/port.rs.

struct PortId
{
    static constexpr uint32_t k_DirBit = 31;
    static constexpr uint32_t k_DirMask = 1u << k_DirBit;
    static constexpr uint32_t k_IndexMask = ~k_DirMask;

    uint32_t    _Id{0xFFFF'FFFF};

    constexpr PortId( void) noexcept = default;
    constexpr explicit PortId( uint32_t raw) noexcept : _Id( raw) {}

    static constexpr PortId In( uint32_t index) noexcept
    {
        return PortId{index & k_IndexMask};
    }

    static constexpr PortId Out( uint32_t index) noexcept
    {
        return PortId{( index & k_IndexMask) | k_DirMask};
    }

    constexpr bool IsOut( void) const noexcept
    {
        return ( _Id & k_DirMask) != 0;
    }

    constexpr bool IsIn( void) const noexcept
    {
        return !IsOut();
    }

    constexpr uint32_t Index( void) const noexcept
    {
        return _Id & k_IndexMask;
    }

    constexpr PortDir Dir( void) const noexcept
    {
        return IsOut() ? PortDir::Out : PortDir::In;
    }

    constexpr bool operator==( const PortId& other) const noexcept = default;
    constexpr bool operator<( const PortId& other) const noexcept { return _Id < other._Id; }
};

//-------------------------------------------------------------------------------------------------
// Port type representation.

enum class PortTypeKind
{
    Bool,
    U8Val,
    U16Val,
    U32Val,
    U64Val,
    Custom,
};

struct PortType
{
    PortTypeKind    _Kind{PortTypeKind::Bool};
    uint32_t        _CustomBits{0};

    static constexpr PortType Bool( void)   noexcept { return PortType{PortTypeKind::Bool, 1}; }
    static constexpr PortType U8Val( void)  noexcept { return PortType{PortTypeKind::U8Val, 8}; }
    static constexpr PortType U16Val( void) noexcept { return PortType{PortTypeKind::U16Val, 16}; }
    static constexpr PortType U32Val( void) noexcept { return PortType{PortTypeKind::U32Val, 32}; }
    static constexpr PortType U64Val( void) noexcept { return PortType{PortTypeKind::U64Val, 64}; }
    static constexpr PortType Custom( uint32_t bits) noexcept { return PortType{PortTypeKind::Custom, bits}; }

    constexpr uint32_t Bits( void) const noexcept
    {
        switch ( _Kind) {
        case PortTypeKind::Bool:   return 1;
        case PortTypeKind::U8Val:  return 8;
        case PortTypeKind::U16Val: return 16;
        case PortTypeKind::U32Val: return 32;
        case PortTypeKind::U64Val: return 64;
        case PortTypeKind::Custom: return _CustomBits;
        default:                   return 0;
        }
    }

    constexpr uint32_t TypeSize( void) const noexcept
    {
        switch ( _Kind) {
        case PortTypeKind::Bool:
        case PortTypeKind::U8Val:
        case PortTypeKind::U16Val:
        case PortTypeKind::U32Val: return 1;
        case PortTypeKind::U64Val: return 2;
        case PortTypeKind::Custom: return ( _CustomBits + 31) / 32;
        default:                   return 1;
        }
    }

    constexpr uint64_t Mask( void) const noexcept
    {
        switch ( _Kind) {
        case PortTypeKind::Bool:   return 1ull;
        case PortTypeKind::U8Val:  return 0xFFull;
        case PortTypeKind::U16Val: return 0xFFFFull;
        case PortTypeKind::U32Val: return 0xFFFF'FFFFull;
        case PortTypeKind::U64Val: return ~0ull;
        case PortTypeKind::Custom: {
            if ( _CustomBits >= 64) return ~0ull;
            if ( _CustomBits == 0) return 0ull;
            return ( 1ull << _CustomBits) - 1ull;
        }
        default: return ~0ull;
        }
    }

    constexpr bool operator==( const PortType& other) const noexcept = default;
};

//-------------------------------------------------------------------------------------------------
// Port descriptor.
// Modeled directly from Kosh rube/port.rs: { _Name, _Type, _Owner }.

struct PortDesc
{
    std::string _Name{};
    PortType    _Type{PortType::Bool()};
    ModuleId    _Owner{};

    PortDesc( void) = default;

    PortDesc( std::string name, PortType portType, ModuleId owner = ModuleId{})
        : _Name( std::move( name)),
          _Type( portType),
          _Owner( owner)
    {
    }

    PortDesc( const char* name, PortType portType = PortType::Bool(), ModuleId owner = ModuleId{})
        : _Name( name ? name : ""),
          _Type( portType),
          _Owner( owner)
    {
    }

    static PortDesc Bool( std::string name)
    {
        return PortDesc( std::move( name), PortType::Bool());
    }

    static PortDesc U32( std::string name)
    {
        return PortDesc( std::move( name), PortType::U32Val());
    }
};

} // namespace trellis::rube

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_RUBE_PORT_H

