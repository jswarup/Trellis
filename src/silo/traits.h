#ifndef TRELLIS_SILO_TRAITS_H
#define TRELLIS_SILO_TRAITS_H

//-------------------------------------------------------------------------------------------------

#include <cstdint>
#include <cstddef>
#include <concepts>
#include <type_traits>
#include <utility>
#include <array>
#include <atomic>
#include <new>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// VTable singleton — one static VTable instance per (Trait, Object) pair.

template < typename TTrait, typename TObj>
struct ObjVTable
{
    static inline const typename TTrait::VTable s_VTable = TTrait::template Bind< TObj>();

    static constexpr const typename TTrait::VTable* Get( void) noexcept
    {
        return &s_VTable;
    }
};

//-------------------------------------------------------------------------------------------------
// Forward Declarations & Type Traits

template < typename TTrait>
struct TRef;

template < typename TTrait>
struct MTRef;

namespace detail {

template < typename T>
struct is_tref : std::false_type {};

template < typename TTrait>
struct is_tref< TRef< TTrait>> : std::true_type {};

template < typename T>
struct is_mtref : std::false_type {};

template < typename TTrait>
struct is_mtref< MTRef< TTrait>> : std::true_type {};

} // namespace detail

template < typename T>
inline constexpr bool is_tref_v = detail::is_tref< std::decay_t< T>>::value;

template < typename T>
inline constexpr bool is_mtref_v = detail::is_mtref< std::decay_t< T>>::value;

//-------------------------------------------------------------------------------------------------
// TRef — non-owning immutable fat pointer. 16 bytes, trivially copyable.

template < typename TTrait>
struct TRef
{
    using VT = typename TTrait::VTable;

    const void*         _Ptr{nullptr};
    const VT*           _Ops{nullptr};

    constexpr TRef( void) noexcept = default;

template < typename T>
        requires ( !is_tref_v< T> && !is_mtref_v< T>)
    constexpr TRef( const T& obj) noexcept
        : _Ptr( &obj),
          _Ops( ObjVTable< TTrait, T>::Get())
    {
    }

template < typename TSuperTrait>
        requires ( !std::same_as< TSuperTrait, TTrait> && std::is_base_of_v< VT, typename TSuperTrait::VTable>)
    constexpr TRef( const TRef< TSuperTrait>& other) noexcept
        : _Ptr( other._Ptr),
          _Ops( static_cast< const VT*>( other._Ops))
    {
    }

template < typename TSuperTrait>
        requires std::is_base_of_v< VT, typename TSuperTrait::VTable>
    constexpr TRef( const MTRef< TSuperTrait>& other) noexcept
        : _Ptr( other._Ptr),
          _Ops( static_cast< const VT*>( other._Ops))
    {
    }

    constexpr TRef( const void* ptr, const VT* ops) noexcept
        : _Ptr( ptr),
          _Ops( ops)
    {
    }

    constexpr bool IsValid( void) const noexcept
    {
        return _Ptr && _Ops;
    }

    constexpr explicit operator bool( void) const noexcept
    {
        return IsValid();
    }

    constexpr const VT* operator->( void) const noexcept
    {
        return _Ops;
    }

template < auto Fn, typename... TArgs>
    constexpr decltype( auto) Invoke( TArgs&&... args) const
    {
        return ( _Ops->*Fn)( _Ptr, std::forward< TArgs>( args)...);
    }

template < typename T>
    const T* As( void) const noexcept
    {
        return ( _Ops == ObjVTable< TTrait, T>::Get()) ? static_cast< const T*>( _Ptr) : nullptr;
    }

template < typename TSubTrait>
        requires ( !std::same_as< TSubTrait, TTrait> && std::is_base_of_v< typename TSubTrait::VTable, VT>)
    constexpr TRef< TSubTrait> AsSub( void) const noexcept
    {
        return { _Ptr, static_cast< const typename TSubTrait::VTable*>( _Ops) };
    }
};

//-------------------------------------------------------------------------------------------------
// MTRef — non-owning mutable fat pointer. 16 bytes, trivially copyable.

template < typename TTrait>
struct MTRef
{
    using VT = typename TTrait::VTable;

    void*               _Ptr{nullptr};
    const VT*           _Ops{nullptr};

    constexpr MTRef( void) noexcept = default;

template < typename T>
        requires ( !is_mtref_v< T> && !is_tref_v< T>)
    constexpr MTRef( T& obj) noexcept
        : _Ptr( &obj),
          _Ops( ObjVTable< TTrait, T>::Get())
    {
    }

template < typename TSuperTrait>
        requires ( !std::same_as< TSuperTrait, TTrait> && std::is_base_of_v< VT, typename TSuperTrait::VTable>)
    constexpr MTRef( const MTRef< TSuperTrait>& other) noexcept
        : _Ptr( other._Ptr),
          _Ops( static_cast< const VT*>( other._Ops))
    {
    }

    constexpr MTRef( void* ptr, const VT* ops) noexcept
        : _Ptr( ptr),
          _Ops( ops)
    {
    }

    constexpr bool IsValid( void) const noexcept
    {
        return _Ptr && _Ops;
    }

    constexpr explicit operator bool( void) const noexcept
    {
        return IsValid();
    }

    constexpr const VT* operator->( void) const noexcept
    {
        return _Ops;
    }

    constexpr operator TRef< TTrait>( void) const noexcept
    {
        return { _Ptr, _Ops };
    }

template < auto Fn, typename... TArgs>
    constexpr decltype( auto) Invoke( TArgs&&... args) const
    {
        return ( _Ops->*Fn)( _Ptr, std::forward< TArgs>( args)...);
    }

template < typename T>
    T* As( void) const noexcept
    {
        return ( _Ops == ObjVTable< TTrait, T>::Get()) ? static_cast< T*>( _Ptr) : nullptr;
    }

template < typename TSubTrait>
        requires ( !std::same_as< TSubTrait, TTrait> && std::is_base_of_v< typename TSubTrait::VTable, VT>)
    constexpr MTRef< TSubTrait> AsSubMut( void) const noexcept
    {
        return { _Ptr, static_cast< const typename TSubTrait::VTable*>( _Ops) };
    }
};

//-------------------------------------------------------------------------------------------------
// TraitMeta — consolidated metadata (VTable, Destructor, Move, Size, Alignment) and TypeId dispatch.

template < typename TTrait, size_t TMaxTypes = 64>
struct TraitMeta
{
    using VT = typename TTrait::VTable;
    static constexpr size_t k_MaxTypes = TMaxTypes;

    uint32_t            _TypeId{0};
    const VT*           _Ops{nullptr};
    void                (*_Destruct)( void*) noexcept{nullptr};
    void                (*_Move)( void* src, void* dst) noexcept{nullptr};
    size_t              _Size{0};
    size_t              _Align{0};

    static inline std::array< const TraitMeta*, k_MaxTypes> s_Table{};
    static inline std::atomic< uint32_t> s_Count{0};

    static uint32_t NextId( void) noexcept
    {
        return s_Count.fetch_add( 1, std::memory_order_relaxed);
    }

    constexpr bool IsInline( size_t cap) const noexcept
    {
        return _Size <= cap && _Align <= alignof( std::max_align_t);
    }

template < typename TObj>
    static const TraitMeta* For( void) noexcept;

template < typename TObj>
    static uint32_t Id( void) noexcept;

    static size_t Count( void) noexcept
    {
        return s_Count.load( std::memory_order_relaxed);
    }

    static const TraitMeta* Get( uint32_t typeId) noexcept
    {
        return ( typeId < k_MaxTypes) ? s_Table[typeId] : nullptr;
    }

    static const VT* VTable( uint32_t typeId) noexcept
    {
        const TraitMeta* m = Get( typeId);
        return m ? m->_Ops : nullptr;
    }

    static TRef< TTrait> Ref( const void* ptr, uint32_t typeId) noexcept
    {
        return { ptr, VTable( typeId) };
    }

    static MTRef< TTrait> MutRef( void* ptr, uint32_t typeId) noexcept
    {
        return { ptr, VTable( typeId) };
    }

template < typename TObj, typename... TArgs>
    static uint32_t Emplace( void* dest, TArgs&&... args)
    {
        ::new ( dest) TObj( std::forward< TArgs>( args)...);
        return Id< TObj>();
    }

    static void Destroy( void* ptr, uint32_t typeId) noexcept
    {
        const TraitMeta* m = Get( typeId);
        if ( m && m->_Destruct)
            m->_Destruct( ptr);
    }

    static void Move( void* src, void* dst, uint32_t typeId) noexcept
    {
        const TraitMeta* m = Get( typeId);
        if ( m && m->_Move)
            m->_Move( src, dst);
    }
};

//-------------------------------------------------------------------------------------------------

template < typename TTrait, typename TObj>
struct ObjMeta
{
    static inline const uint32_t s_Id = TraitMeta< TTrait>::NextId();

    static inline const TraitMeta< TTrait> s_Meta {
        ._TypeId   = s_Id,
        ._Ops      = ObjVTable< TTrait, TObj>::Get(),
        ._Destruct = []( void* p) noexcept {
            static_cast< TObj*>( p)->~TObj();
        },
        ._Move     = []( void* s, void* d) noexcept {
            ::new ( d) TObj( std::move( *static_cast< TObj*>( s)));
            static_cast< TObj*>( s)->~TObj();
        },
        ._Size     = sizeof( TObj),
        ._Align    = alignof( TObj)
    };

    static const TraitMeta< TTrait>* Get( void) noexcept
    {
        static const bool s_Registered = []() noexcept {
            if ( s_Id < TraitMeta< TTrait>::k_MaxTypes)
                TraitMeta< TTrait>::s_Table[s_Id] = &s_Meta;
            return true;
        }();
        (void)s_Registered;
        return &s_Meta;
    }

    static uint32_t Id( void) noexcept
    {
        Get();
        return s_Id;
    }
};

//-------------------------------------------------------------------------------------------------

template < typename TTrait, size_t TMaxTypes>
template < typename TObj>
const TraitMeta< TTrait, TMaxTypes>* TraitMeta< TTrait, TMaxTypes>::For( void) noexcept
{
    return ObjMeta< TTrait, TObj>::Get();
}

//-------------------------------------------------------------------------------------------------

template < typename TTrait, size_t TMaxTypes>
template < typename TObj>
uint32_t TraitMeta< TTrait, TMaxTypes>::Id( void) noexcept
{
    return ObjMeta< TTrait, TObj>::Id();
}

//-------------------------------------------------------------------------------------------------
// TPtr — owning fat pointer with inline SBO (Small Buffer Optimization).

template < typename TTrait, size_t TInlineCap = 48>
class TPtr
{
    using VT = typename TTrait::VTable;

    alignas( std::max_align_t) std::byte _Buf[TInlineCap]{};
    void*               _Ptr{nullptr};
    const TraitMeta< TTrait>* _Meta{nullptr};

    bool IsInline( void) const noexcept
    {
        return _Meta && _Meta->IsInline( TInlineCap);
    }

    void Destroy( void) noexcept
    {
        if ( !_Ptr)
            return;

        if ( _Meta && _Meta->_Destruct)
            _Meta->_Destruct( _Ptr);

        if ( _Meta && !IsInline())
            ::operator delete( _Ptr);

        _Ptr  = nullptr;
        _Meta = nullptr;
    }

public:
    TPtr( void) noexcept = default;

template < typename T>
        requires ( !std::same_as< std::decay_t< T>, TPtr>)
    TPtr( T&& val)
    {
        using D = std::decay_t< T>;
        _Meta = TraitMeta< TTrait>::template For< D>();
        if ( IsInline()) {
            _Ptr = _Buf;
        } else {
            _Ptr = ::operator new( sizeof( D), std::align_val_t{ alignof( D) });
        }
        ::new ( _Ptr) D( std::forward< T>( val));
    }

    ~TPtr( void) noexcept
    {
        Destroy();
    }

    TPtr( const TPtr&) = delete;
    TPtr& operator=( const TPtr&) = delete;

    TPtr( TPtr&& o) noexcept
        : _Meta( o._Meta)
    {
        if ( IsInline()) {
            _Ptr = _Buf;
            _Meta->_Move( o._Ptr, _Ptr);
        } else {
            _Ptr = o._Ptr;
        }
        o._Ptr  = nullptr;
        o._Meta = nullptr;
    }

    TPtr& operator=( TPtr&& o) noexcept
    {
        if ( this != &o) {
            Destroy();
            _Meta = o._Meta;
            if ( IsInline()) {
                _Ptr = _Buf;
                _Meta->_Move( o._Ptr, _Ptr);
            } else {
                _Ptr = o._Ptr;
            }
            o._Ptr  = nullptr;
            o._Meta = nullptr;
        }
        return *this;
    }

    bool IsValid( void) const noexcept
    {
        return _Ptr && _Meta && _Meta->_Ops;
    }

    explicit operator bool( void) const noexcept
    {
        return IsValid();
    }

    const VT* operator->( void) const noexcept
    {
        return _Meta ? _Meta->_Ops : nullptr;
    }

template < auto Fn, typename... TArgs>
    decltype( auto) Invoke( TArgs&&... args) const
    {
        return ( _Meta->_Ops->*Fn)( _Ptr, std::forward< TArgs>( args)...);
    }

    TRef< TTrait> AsRef( void) const noexcept
    {
        return { _Ptr, _Meta ? _Meta->_Ops : nullptr };
    }

    MTRef< TTrait> AsMutRef( void) noexcept
    {
        return { _Ptr, _Meta ? _Meta->_Ops : nullptr };
    }

template < typename T>
    const T* As( void) const noexcept
    {
        return ( _Meta && _Meta->_Ops == ObjVTable< TTrait, T>::Get()) ? static_cast< const T*>( _Ptr) : nullptr;
    }
};

//-------------------------------------------------------------------------------------------------
// TraitBundle — variadic zero-overhead composition of multiple independent traits.

template < typename... TTraits>
struct TraitBundle
{
    struct VTable : TTraits::VTable... {};

template < typename T>
        requires ( ( requires { TTraits::template Bind< T>(); } ) && ... )
    static constexpr VTable Bind( void) noexcept
    {
        return VTable{ { TTraits::template Bind< T>() }... };
    }
};

//-------------------------------------------------------------------------------------------------

template < typename... TTraits>
using BundleRef = TRef< TraitBundle< TTraits...>>;

template < typename... TTraits>
using MutBundleRef = MTRef< TraitBundle< TTraits...>>;

template < typename... TTraits>
using BundlePtr = TPtr< TraitBundle< TTraits...>>;

} // namespace trellis::silo

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SILO_TRAITS_H
