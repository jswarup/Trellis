// mod.rs ---------------------------------------------------------------------------------------------------------
use crate::symph::compshade::{
    Collatz, CollatzElem, DoubleElem, HashToFloat, PointCloudElem, VectorAddElem, WangHash,
};
use crate::symph::vertshade::{CameraUniforms, Vec3, VertexTransformPos};
use crate::{
    segue_assert, segue_assert_eq, segue_console_test, segue_example_test, segue_println,
    segue_test,
};

//-------------------------------------------------------------------------------------------------

// Symph Compute Shader Tests
segue_test!( Symph, WangHashDeterminism, |ctx| {
    let  h0 = WangHash( 0);
    let  h1 = WangHash( 1);
    segue_assert!( ctx, h0 != h1);
    segue_assert_eq!( ctx, WangHash( 0), h0);
    segue_assert_eq!( ctx, WangHash( 1), h1);
});
segue_test!( Symph, HashToFloatRange, |ctx| {
    for seed in 0..100
    {
        let  h = WangHash( seed);
        let  f = HashToFloat( h);
        segue_assert!( ctx, ( 0.0..1.0).contains( &f));
    }
});
segue_test!( Symph, CollatzKnownValues, |ctx| {
    segue_assert_eq!( ctx, Collatz( 0), u32::MAX);
    segue_assert_eq!( ctx, Collatz( 1), 0);
    segue_assert_eq!( ctx, Collatz( 2), 1);
    segue_assert_eq!( ctx, Collatz( 3), 7);
    segue_assert_eq!( ctx, Collatz( 6), 8);
    segue_assert_eq!( ctx, Collatz( 7), 16);
});
segue_test!( Symph, ElementWiseKernels, |ctx| {
    let  mut dbl = [1.0f32, 2.5, -3.0];
    DoubleElem( 0, &mut dbl);
    DoubleElem( 1, &mut dbl);
    DoubleElem( 2, &mut dbl);
    DoubleElem( 99, &mut dbl);                                         // out-of-bounds safe
    segue_assert_eq!( ctx, dbl[0], 2.0);
    segue_assert_eq!( ctx, dbl[1], 5.0);
    segue_assert_eq!( ctx, dbl[2], -6.0);
    let  a = [1.0f32, 2.0, 3.0];
    let  b = [10.0f32, 20.0, 30.0];
    let  mut out_add = [0.0f32; 3];
    for i in 0..3
    {
        VectorAddElem( i, &a, &b, &mut out_add);
    }
    segue_assert_eq!( ctx, out_add[0], 11.0);
    segue_assert_eq!( ctx, out_add[1], 22.0);
    segue_assert_eq!( ctx, out_add[2], 33.0);
    let  inp = [1u32, 2, 3, 6];
    let  mut out_col = [0u32; 4];
    for i in 0..4
    {
        CollatzElem( i, &inp, &mut out_col);
    }
    segue_assert_eq!( ctx, out_col[0], 0);
    segue_assert_eq!( ctx, out_col[1], 1);
    segue_assert_eq!( ctx, out_col[2], 7);
    segue_assert_eq!( ctx, out_col[3], 8);
    let  mut pt = [0.0f32; 4];
    PointCloudElem( 0, &mut pt);
    segue_assert!( ctx, pt[0] >= -20.0 && pt[0] <= 20.0);
    segue_assert!( ctx, pt[1] >= -20.0 && pt[1] <= 20.0);
    segue_assert!( ctx, pt[2] >= -20.0 && pt[2] <= 20.0);
    segue_assert_eq!( ctx, pt[3], 1.0);
});

//-------------------------------------------------------------------------------------------------

// Symph Vertex Shader Tests
segue_test!( Symph, VertexTransformPosProjection, |ctx| {
    let  cam = CameraUniforms::default();
    let  pos = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let  res = VertexTransformPos( &pos, &cam);
    segue_assert!( ctx, ( res.clipPos.x).abs() < 1e-4);
    segue_assert!( ctx, ( res.clipPos.y).abs() < 1e-4);
    segue_assert_eq!( ctx, res.clipPos.w, 1.0);
    segue_assert!( ctx, res.depthFactor > 0.0);
    segue_assert!( ctx, res.ptSize > 0.0);
});

//-------------------------------------------------------------------------------------------------

// Console & Example Tests
segue_console_test!( Symph, SymphConsoleReport, |ctx| {
    let  h = WangHash( 42);
    let  f = HashToFloat( h);
    let  c = Collatz( 27);
    segue_println!(
        ctx,
        "         [Symph] WangHash(42) = 0x{:08X}, HashToFloat = {:.4}, Collatz(27) = {}",
        h,
        f,
        c
    );
    segue_assert!( ctx, c > 0);
});
segue_example_test!( Symph, SymphPipelineExample, |ctx| {
    let  cam = CameraUniforms::default();
    let  p = Vec3 {
        x: 10.0,
        y: 5.0,
        z: 20.0,
    };
    let  res = VertexTransformPos( &p, &cam);
    segue_println!(
        ctx,
        "         [Example] Vertex transformed to clipPos=({:.3}, {:.3}, {:.3}), ptSize={:.2}",
        res.clipPos.x,
        res.clipPos.y,
        res.clipPos.z,
        res.ptSize
    );
    segue_assert!( ctx, res.ptSize > 0.0);
});
