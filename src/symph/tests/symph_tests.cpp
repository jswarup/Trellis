//-------------------------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "symph/symph.h"

#include <cmath>
#include <vector>

using namespace trellis::symph;

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Symph, WangHashAndFloat)
{
    uint32_t    h0 = WangHash( 0);
    uint32_t    h1 = WangHash( 1);
    JEEVES_ASSERT_NE( h0, h1);

    // Deterministic check
    JEEVES_ASSERT_EQ( WangHash( 42), WangHash( 42));

    float       f0 = HashToFloat( h0);
    float       f1 = HashToFloat( h1);
    JEEVES_ASSERT( f0 >= 0.0f && f0 < 1.0f);
    JEEVES_ASSERT( f1 >= 0.0f && f1 < 1.0f);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Symph, Collatz)
{
    JEEVES_ASSERT_EQ( Collatz( 0), UINT32_MAX);
    JEEVES_ASSERT_EQ( Collatz( 1), 0u);
    JEEVES_ASSERT_EQ( Collatz( 2), 1u);
    JEEVES_ASSERT_EQ( Collatz( 3), 7u); // 3 -> 10 -> 5 -> 16 -> 8 -> 4 -> 2 -> 1 (7 steps)
    JEEVES_ASSERT_EQ( Collatz( 4), 2u);
    JEEVES_ASSERT_EQ( Collatz( 6), 8u);

    // Overflow guard test (n >= 0x55555555 and odd)
    JEEVES_ASSERT_EQ( Collatz( 0x55555555), UINT32_MAX);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Symph, ElementWiseKernels)
{
    // DoubleElem
    float       doubles[4] = {1.0f, 2.5f, -3.0f, 4.0f};
    for ( size_t i = 0; i < 4; ++i) {
        DoubleElem( i, doubles, 4);
    }
    JEEVES_ASSERT_EQ( doubles[0], 2.0f);
    JEEVES_ASSERT_EQ( doubles[1], 5.0f);
    JEEVES_ASSERT_EQ( doubles[2], -6.0f);
    JEEVES_ASSERT_EQ( doubles[3], 8.0f);

    // VectorAddElem
    const float a[3] = {1.0f, 2.0f, 3.0f};
    const float b[3] = {10.0f, 20.0f, 30.0f};
    float       out[3] = {0.0f, 0.0f, 0.0f};
    for ( size_t i = 0; i < 3; ++i) {
        VectorAddElem( i, a, b, out, 3);
    }
    JEEVES_ASSERT_EQ( out[0], 11.0f);
    JEEVES_ASSERT_EQ( out[1], 22.0f);
    JEEVES_ASSERT_EQ( out[2], 33.0f);

    // CollatzElem
    const uint32_t      cIn[3] = {1, 3, 0};
    uint32_t            cOut[3] = {99, 99, 99};
    for ( size_t i = 0; i < 3; ++i) {
        CollatzElem( i, cIn, cOut, 3);
    }
    JEEVES_ASSERT_EQ( cOut[0], 0u);
    JEEVES_ASSERT_EQ( cOut[1], 7u);
    JEEVES_ASSERT_EQ( cOut[2], UINT32_MAX);

    // PointCloudElem
    float       pts[8] = {0};
    PointCloudElem( 0, pts, 8);
    PointCloudElem( 1, pts, 8);
    JEEVES_ASSERT( pts[0] >= -20.0f && pts[0] <= 20.0f);
    JEEVES_ASSERT( pts[1] >= -20.0f && pts[1] <= 20.0f);
    JEEVES_ASSERT( pts[2] >= -20.0f && pts[2] <= 20.0f);
    JEEVES_ASSERT_EQ( pts[3], 1.0f);
    JEEVES_ASSERT_EQ( pts[7], 1.0f);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Symph, CameraTransformAndFrustum)
{
    const float inPoints[3] = {0.0f, 0.0f, 0.0f};
    const float camParams[13] = {
        0.0f, 0.0f, // rotX, rotY
        1.0f,       // zoom
        0.0f, 0.0f, // panX, panY
        60.0f,      // fov
        500.0f,     // distance
        800.0f, 600.0f, // width, height
        0.0f, 0.0f, 0.0f, // cx, cy, cz
        1.0f        // scaleNorm
    };
    float       proj[6] = {0};
    CameraTransformElem( 0, inPoints, 3, camParams, 13, proj, 6);

    JEEVES_ASSERT_EQ( proj[0], 400.0f); // width / 2
    JEEVES_ASSERT_EQ( proj[1], 300.0f); // height / 2
    JEEVES_ASSERT( proj[2] > 0.0f);     // radius
    JEEVES_ASSERT( proj[3] > 0.0f);     // coreRadius
    JEEVES_ASSERT( proj[4] > 0.0f);     // alpha
    JEEVES_ASSERT( proj[5] > 0.0f);     // depthFactor

    // Frustum cull
    // 6 planes: x=1, x=-1, y=1, y=-1, z=1, z=-1 with distance 10
    float planes[24] = {
         1.0f,  0.0f,  0.0f, 10.0f,
        -1.0f,  0.0f,  0.0f, 10.0f,
         0.0f,  1.0f,  0.0f, 10.0f,
         0.0f, -1.0f,  0.0f, 10.0f,
         0.0f,  0.0f,  1.0f, 10.0f,
         0.0f,  0.0f, -1.0f, 10.0f,
    };
    uint32_t    visible = 0;
    FrustumCullElem( 0, inPoints, 3, planes, 24, &visible, 1);
    JEEVES_ASSERT_EQ( visible, 1u);

    // Far outside point
    const float outPoints[3] = {100.0f, 0.0f, 0.0f};
    FrustumCullElem( 0, outPoints, 3, planes, 24, &visible, 1);
    JEEVES_ASSERT_EQ( visible, 0u);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Symph, Vertshade)
{
    CameraUniforms      cam;
    cam._Width = 800.0f;
    cam._Height = 600.0f;
    cam._Distance = 500.0f;
    cam._Fov = 60.0f;
    cam._Zoom = 1.0f;

    Vec3                pos{0.0f, 0.0f, 0.0f};
    auto                res = VertexTransformPos( pos, cam);
    JEEVES_ASSERT_EQ( res.clipPos.x, 0.0f);
    JEEVES_ASSERT_EQ( res.clipPos.y, 0.0f);
    JEEVES_ASSERT_EQ( res.clipPos.w, 1.0f);

    Vec4                baseCol{1.0f, 0.0f, 0.0f, 1.0f};
    Vec2                centerCoord{0.5f, 0.5f};
    Vec4                shadedCenter = FragmentPointColor( centerCoord, baseCol);
    JEEVES_ASSERT( shadedCenter.w > 0.0f);

    Vec2                outsideCoord{1.0f, 1.0f};
    Vec4                shadedOutside = FragmentPointColor( outsideCoord, baseCol);
    JEEVES_ASSERT_EQ( shadedOutside.w, 0.0f);
}

//-------------------------------------------------------------------------------------------------

