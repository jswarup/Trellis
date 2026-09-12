#ifndef TRELLIS_SYMPH_VERTSHADE_H
#define TRELLIS_SYMPH_VERTSHADE_H

//-------------------------------------------------------------------------------------------------

#include <algorithm>
#include <cmath>
#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::symph {

//-------------------------------------------------------------------------------------------------

struct Vec2
{
    float       x{0.0f};
    float       y{0.0f};
};

struct Vec3
{
    float       x{0.0f};
    float       y{0.0f};
    float       z{0.0f};
};

struct Vec4
{
    float       x{0.0f};
    float       y{0.0f};
    float       z{0.0f};
    float       w{0.0f};
};

//-------------------------------------------------------------------------------------------------
// Camera uniform parameter block for dedicated vertex & fragment rendering pipelines.

struct CameraUniforms
{
    float       _RotX{0.0f};
    float       _RotY{0.0f};
    float       _Zoom{1.0f};
    float       _PanX{0.0f};
    float       _PanY{0.0f};
    float       _Fov{60.0f};
    float       _Distance{500.0f};
    float       _Width{1920.0f};
    float       _Height{1080.0f};
    float       _CenterX{0.0f};
    float       _CenterY{0.0f};
    float       _CenterZ{0.0f};
    float       _ScaleNorm{1.0f};
};

struct VertexTransformResult
{
    Vec4        clipPos{};
    float       ptSize{0.0f};
    float       depthFactor{0.0f};
};

//-------------------------------------------------------------------------------------------------
// Transforms a 3D point in world space to NDC coordinates and returns (clip_pos, point_size, depth_factor).

inline VertexTransformResult VertexTransformPos( const Vec3& pos, const CameraUniforms& cam) noexcept
{
    const float nx = ( pos.x - cam._CenterX) * cam._ScaleNorm;
    const float ny = ( pos.y - cam._CenterY) * cam._ScaleNorm;
    const float nz = ( pos.z - cam._CenterZ) * cam._ScaleNorm;

    const float cosY = std::cos( cam._RotY);
    const float sinY = std::sin( cam._RotY);
    const float x1 = nx * cosY + nz * sinY;
    const float z1 = -nx * sinY + nz * cosY;

    const float cosX = std::cos( cam._RotX);
    const float sinX = std::sin( cam._RotX);
    const float y2 = ny * cosX - z1 * sinX;
    const float z2 = ny * sinX + z1 * cosX;

    const float denom = cam._Distance + z2;
    const float w = ( denom > 1e-4f) ? denom : 1e-4f;
    const float scale = ( cam._Fov * cam._Zoom) / w;

    const float projX = cam._Width / 2.0f + cam._PanX + x1 * scale;
    const float projY = cam._Height / 2.0f + cam._PanY - y2 * scale;

    const float ndcX = ( projX / cam._Width) * 2.0f - 1.0f;
    const float ndcY = ( projY / cam._Height) * 2.0f - 1.0f;
    const float ndcZ = z2 / 400.0f;

    const float depthFactor = std::max( 0.3f, std::min( 1.0f, ( 300.0f - z2) / 400.0f));
    const float ptSize = 6.0f + depthFactor * 8.0f;

    return VertexTransformResult{
        .clipPos = Vec4{ndcX, ndcY, ndcZ, 1.0f},
        .ptSize = ptSize,
        .depthFactor = depthFactor
    };
}

//-------------------------------------------------------------------------------------------------
// Evaluates point-sprite fragment shading: circular falloff, glowing white core, and depth alpha.

inline Vec4 FragmentPointColor( const Vec2& pointCoord, const Vec4& baseColor) noexcept
{
    const float deltaX = pointCoord.x - 0.5f;
    const float deltaY = pointCoord.y - 0.5f;
    const float distSq = deltaX * deltaX + deltaY * deltaY;
    if ( distSq > 0.25f) {
        return Vec4{0.0f, 0.0f, 0.0f, 0.0f};
    }

    const float dist = std::sqrt( distSq) * 2.0f;
    const float alpha = std::max( 0.0f, 1.0f - dist) * baseColor.w;
    const float core = ( dist < 0.3f) ? ( 1.0f - dist / 0.3f) : 0.0f;
    const float r = baseColor.x + core * ( 1.0f - baseColor.x);
    const float g = baseColor.y + core * ( 1.0f - baseColor.y);
    const float b = baseColor.z + core * ( 1.0f - baseColor.z);
    return Vec4{r, g, b, alpha};
}

} // namespace trellis::symph

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SYMPH_VERTSHADE_H

