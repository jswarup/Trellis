// compshade.h ----------------------------------------------------------------------------------------------------
#pragma once

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::symph {

//-------------------------------------------------------------------------------------------------
// Wang hash — fast, deterministic integer hash for pseudo-random number generation.

inline constexpr uint32_t WangHash( uint32_t seed) noexcept
{
    seed = ( seed ^ 61) ^ ( seed >> 16);
    seed = seed * 9;
    seed = seed ^ ( seed >> 4);
    seed = seed * 0x27d4eb2d;
    seed = seed ^ ( seed >> 15);
    return seed;
}

//-------------------------------------------------------------------------------------------------
// Maps a hash value to a float in [0.0, 1.0) by masking to 24 bits and dividing by 2^24.

inline constexpr float HashToFloat( uint32_t h) noexcept
{
    return static_cast< float>( h & 0x00FF'FFFF) / 16777216.0f;
}

//-------------------------------------------------------------------------------------------------
// Computes the number of Collatz steps for an integer. Returns UINT32_MAX on 0 or overflow.

inline constexpr uint32_t Collatz( uint32_t n) noexcept
{
    if ( n == 0)
        return UINT32_MAX;

    uint32_t    i = 0;
    while ( n != 1) {
        if ( ( n % 2) == 0) {
            n /= 2;
        } else {
            if ( n >= 0x5555'5555)
                return UINT32_MAX;
            n = 3 * n + 1;
        }
        ++i;
    }
    return i;
}

//-------------------------------------------------------------------------------------------------
// In-place element doubling.

inline void DoubleElem( size_t idx, float* data, size_t len) noexcept
{
    if ( idx < len) {
        data[idx] *= 2.0f;
    }
}

//-------------------------------------------------------------------------------------------------
// Element-wise vector addition: out = a + b.

inline void VectorAddElem( size_t idx, const float* a, const float* b, float* out, size_t len) noexcept
{
    if ( idx < len) {
        out[idx] = a[idx] + b[idx];
    }
}

//-------------------------------------------------------------------------------------------------
// Element-wise Collatz sequence computation.

inline void CollatzElem( size_t idx, const uint32_t* in, uint32_t* out, size_t len) noexcept
{
    if ( idx < len) {
        out[idx] = Collatz( in[idx]);
    }
}

//-------------------------------------------------------------------------------------------------
// Element-wise 3D point cloud generation in [-20, 20]^3.

inline void PointCloudElem( size_t idx, float* out, size_t len) noexcept
{
    const size_t        base = idx * 4;
    if ( base + 3 < len) {
        const uint32_t  hx = WangHash( static_cast< uint32_t>( idx * 3 + 0));
        const uint32_t  hy = WangHash( static_cast< uint32_t>( idx * 3 + 1));
        const uint32_t  hz = WangHash( static_cast< uint32_t>( idx * 3 + 2));

        const float     x = HashToFloat( hx) * 40.0f - 20.0f;
        const float     y = HashToFloat( hy) * 40.0f - 20.0f;
        const float     z = HashToFloat( hz) * 40.0f - 20.0f;

        out[base + 0] = x;
        out[base + 1] = y;
        out[base + 2] = z;
        out[base + 3] = 1.0f;
    }
}

//-------------------------------------------------------------------------------------------------
// Element-wise 3D point cloud camera transformation and perspective projection.
// Reads 3D point (x, y, z) and camera uniform parameters, writes 6 projected values:
// [screen_x, screen_y, radius, core_radius, alpha, depth_factor].

inline void CameraTransformElem(
    size_t idx,
    const float* inPoints,
    size_t inLen,
    const float* camParams,
    size_t camLen,
    float* outProjected,
    size_t outLen) noexcept
{
    const size_t        inBase = idx * 3;
    const size_t        outBase = idx * 6;

    if ( inBase + 2 < inLen && outBase + 5 < outLen && camLen >= 13) {
        const float     x = inPoints[inBase + 0];
        const float     y = inPoints[inBase + 1];
        const float     z = inPoints[inBase + 2];

        const float     rotX = camParams[0];
        const float     rotY = camParams[1];
        const float     zoom = camParams[2];
        const float     panX = camParams[3];
        const float     panY = camParams[4];
        const float     fov = camParams[5];
        const float     distance = camParams[6];
        const float     width = camParams[7];
        const float     height = camParams[8];
        const float     cx = camParams[9];
        const float     cy = camParams[10];
        const float     cz = camParams[11];
        const float     scaleNorm = camParams[12];

        const float     nx = ( x - cx) * scaleNorm;
        const float     ny = ( y - cy) * scaleNorm;
        const float     nz = ( z - cz) * scaleNorm;

        const float     cosY = std::cos( rotY);
        const float     sinY = std::sin( rotY);
        const float     x1 = nx * cosY + nz * sinY;
        const float     z1 = -nx * sinY + nz * cosY;

        const float     cosX = std::cos( rotX);
        const float     sinX = std::sin( rotX);
        const float     y2 = ny * cosX - z1 * sinX;
        const float     z2 = ny * sinX + z1 * cosX;

        const float     scale = ( fov * zoom) / ( distance + z2);

        const float     projX = width / 2.0f + panX + x1 * scale;
        const float     projY = height / 2.0f + panY - y2 * scale;

        const float     depthFactor = std::max( 0.3f, std::min( 1.0f, ( 300.0f - z2) / 400.0f));
        const float     radius = 3.0f + depthFactor * 4.0f;
        const float     coreRadius = 1.0f + depthFactor * 1.5f;
        const float     alpha = 0.5f + depthFactor * 0.5f;

        outProjected[outBase + 0] = projX;
        outProjected[outBase + 1] = projY;
        outProjected[outBase + 2] = radius;
        outProjected[outBase + 3] = coreRadius;
        outProjected[outBase + 4] = alpha;
        outProjected[outBase + 5] = depthFactor;
    }
}

//-------------------------------------------------------------------------------------------------
// Tests whether a 3D point is inside the 6 frustum planes.
// Returns 1 if visible (inside all 6 planes), 0 if culled.

inline void FrustumCullElem(
    size_t idx,
    const float* inPoints,
    size_t inLen,
    const float* frustumPlanes,
    size_t planeLen,
    uint32_t* outVisible,
    size_t outLen) noexcept
{
    const size_t        base = idx * 3;
    if ( base + 2 < inLen && idx < outLen && planeLen >= 24) {
        const float     x = inPoints[base + 0];
        const float     y = inPoints[base + 1];
        const float     z = inPoints[base + 2];

        uint32_t        visible = 1;
        for ( size_t p = 0; p < 6; ++p) {
            const size_t        pBase = p * 4;
            const float         a = frustumPlanes[pBase + 0];
            const float         b = frustumPlanes[pBase + 1];
            const float         c = frustumPlanes[pBase + 2];
            const float         d = frustumPlanes[pBase + 3];

            const float         dist = a * x + b * y + c * z + d;
            if ( dist < -0.5f) {
                visible = 0;
                break;
            }
        }

        outVisible[idx] = visible;
    }
}

} // namespace trellis::symph
