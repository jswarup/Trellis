// plot3d.rs --------------------------------------------------------------------------------------
//! 3D surface mesh sampling from Fresco scalar expressions into `GeometryAsset`.

use crate::fenst::fresco_document::{FrescoDocument, FrescoRootKind};
use crate::fenst::geometry::GeometryAsset;
use crate::fresco::{CompileLimits, CompiledProgram, ExprRepos};
use crate::silo::{Arr, Buff, Stash};

//-------------------------------------------------------------------------------------------------

/// Evaluates a 2D Fresco scalar expression $z = f(x, y)$ on a `grid_n x grid_n` mesh
/// and constructs a height-colored `GeometryAsset` for `ViewGeometry`.
pub fn SampleFrescoSurface3d(
    doc: &FrescoDocument,
    root_index: u32,
    x_range: (f64, f64),
    y_range: (f64, f64),
    grid_n: usize,
) -> Result<GeometryAsset, String>
{
    let n = grid_n.clamp(8, 128);
    let root = doc.RootAt(root_index).ok_or_else(|| "Selected Fresco root not found".to_string())?;
    let FrescoRootKind::Scalar { _ExprId } = root.Kind() else {
        return Err("3D surface plotting requires a scalar root".into());
    };
    let expr_id = *_ExprId;
    let repos = doc.Repos();

    let mut positions = Vec::with_capacity(n * n);
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;

    let compiled = CompiledProgram::Compile(repos, Arr::from(&[expr_id]), &CompileLimits::default()).ok();
    let mut ws_opt = compiled.as_ref().map(CompiledProgram::AllocWorkspace);
    let mut x_slot = None;
    let mut y_slot = None;
    if let Some(ref prog) = compiled {
        for v in 0..prog.VarCount() {
            match prog.VarNameAt(v) {
                Some("x") => x_slot = Some(v as usize),
                Some("y") => y_slot = Some(v as usize),
                _ => {}
            }
        }
    }

    for iy in 0..n {
        let ty = iy as f64 / (n - 1) as f64;
        let y = y_range.0 + ty * (y_range.1 - y_range.0);
        for ix in 0..n {
            let tx = ix as f64 / (n - 1) as f64;
            let x = x_range.0 + tx * (x_range.1 - x_range.0);

            let z = eval_2d_point(repos, expr_id, compiled.as_ref(), ws_opt.as_mut(), x_slot, y_slot, x, y);
            let z_clamped = if z.is_finite() { z } else { 0.0 };
            z_min = z_min.min(z_clamped);
            z_max = z_max.max(z_clamped);
            positions.push([x, y, z_clamped]);
        }
    }

    let z_span = (z_max - z_min).max(1e-9);
    let mut colors = Vec::with_capacity(positions.len());
    for pos in &positions {
        let t = ((pos[2] - z_min) / z_span).clamp(0.0, 1.0) as f32;
        colors.push(height_colormap(t));
    }

    let mut triangles = Stash::<[u32; 3]>::WithCapacity(((n - 1) * (n - 1) * 2) as u32);
    let mut edges = Stash::<[u32; 2]>::WithCapacity(((n - 1) * n * 2) as u32);
    for iy in 0..(n - 1) {
        for ix in 0..(n - 1) {
            let a = (iy * n + ix) as u32;
            let b = a + 1;
            let c = ((iy + 1) * n + ix) as u32;
            let d = c + 1;
            triangles.Push([a, b, d]);
            triangles.Push([a, d, c]);
            edges.Push([a, b]);
            edges.Push([a, c]);
        }
    }

    let bounds = (
        [x_range.0, y_range.0, z_min],
        [x_range.1, y_range.1, if (z_max - z_min).abs() < 1e-9 { z_min + 1.0 } else { z_max }],
    );

    return GeometryAsset::FromMeshF64(
        Arr::from(positions.as_slice()),
        Arr::from(colors.as_slice()),
        triangles.IntoBuff(),
        edges.IntoBuff(),
        Buff::New(),
        bounds,
    );
}

fn eval_2d_point(
    repos: &ExprRepos,
    expr_id: u32,
    prog_opt: Option<&CompiledProgram>,
    ws_opt: Option<&mut crate::fresco::CompiledWorkspace>,
    x_slot: Option<usize>,
    y_slot: Option<usize>,
    x: f64,
    y: f64,
) -> f64
{
    if let (Some(prog), Some(ws)) = (prog_opt, ws_opt) {
        let mut inputs = vec![1.0; prog.VarCount() as usize];
        if let Some(sx) = x_slot {
            inputs[sx] = x;
        }
        if let Some(sy) = y_slot {
            inputs[sy] = y;
        }
        if ws.SetInputs(Arr::from(inputs.as_slice())).is_ok()
            && let Ok(vals) = ws.Evaluate(prog)
        {
            return vals[0];
        }
    }
    return repos
        .EvaluateNamed(expr_id, Arr::from(&[("x", x), ("y", y), ("theta", x)]))
        .unwrap_or(0.0);
}

fn height_colormap(t: f32) -> [f32; 4]
{
    // Smooth blue -> cyan -> emerald -> amber -> coral ramp
    let r = (1.5 * t - 0.35).clamp(0.12, 0.96);
    let g = (1.0 - (2.0 * t - 1.0).abs() * 0.65).clamp(0.20, 0.90);
    let b = (1.15 - 1.2 * t).clamp(0.16, 0.95);
    return [r, g, b, 1.0];
}
