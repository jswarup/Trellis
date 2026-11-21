// camera.rs ------------------------------------------------------------------------------------------------------------------

use crate::fenst::cask_scene::ViewBox;
use glam::{Mat4, Quat, Vec3};

//-----------------------------------------------------------------------------------------------------------------------------

/// Geometry is normalized to a unit bounding sphere at import; navigation remains scale independent.
#[derive(Debug, Clone, Copy)]
pub struct ViewCamera {
    _Orientation: Quat,
    _Target: Vec3,
    _Distance: f32,
    _Orthographic: bool,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl Default for ViewCamera {
    fn default() -> Self {
        Self {
            _Orientation: Quat::from_rotation_y(0.6) * Quat::from_rotation_x(-0.3),
            _Target: Vec3::ZERO,
            _Distance: 4.0,
            _Orthographic: false,
        }
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl ViewCamera {
    const FOV: f32 = std::f32::consts::FRAC_PI_4;
    pub fn Orbit(&mut self, dx: f32, dy: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self._Orientation = (Quat::from_rotation_y(-dx * 0.008)
            * self._Orientation
            * Quat::from_rotation_x(-dy * 0.008))
        .normalize();
    }
    pub fn Pan(&mut self, dx: f32, dy: f32, height: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        let scale = 2.0 * self._Distance * (Self::FOV * 0.5).tan() / height.max(1.0);
        self._Target += self._Orientation * Vec3::new(-dx * scale, dy * scale, 0.0);
    }
    pub fn Zoom(&mut self, steps: f32) {
        if steps.is_finite() {
            self._Distance = (self._Distance * (-steps * 0.12).exp()).clamp(0.02, 1000.0);
        }
    }
    pub fn Fit(&mut self, aspect: f32) {
        let halfAngle = ((Self::FOV * 0.5).tan() * aspect.clamp(0.001, 1.0)).atan();
        self._Distance = 1.1 / halfAngle.sin();
        self._Target = Vec3::ZERO;
    }
    pub fn Reset(&mut self, aspect: f32) {
        *self = Self::default();
        self.Fit(aspect);
    }
    pub fn ToggleProjection(&mut self) {
        self._Orthographic = !self._Orthographic;
    }
    pub fn IsOrthographic(&self) -> bool {
        self._Orthographic
    }
    pub fn SetAxis(&mut self, axis: u32) {
        self._Orientation = match axis {
            1 => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            2 => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            _ => Quat::IDENTITY,
        };
    }
    pub fn Matrix(&self, aspect: f32) -> [f32; 16] {
        let aspect = aspect.max(0.001);
        let eye = self._Target + self._Orientation * Vec3::Z * self._Distance;
        let view = Mat4::look_at_rh(eye, self._Target, self._Orientation * Vec3::Y);
        let far = self._Distance + self._Target.length() + 4.0;
        let near = (self._Distance * 0.001).max(0.00001);
        let projection = if self._Orthographic {
            let half = self._Distance * (Self::FOV * 0.5).tan();
            Mat4::orthographic_rh(-half * aspect, half * aspect, -half, half, near, far)
        } else {
            Mat4::perspective_rh(Self::FOV, aspect, near, far)
        };
        (projection * view).to_cols_array()
    }
    pub fn Distance(&self) -> f32 {
        return self._Distance;
    }
    pub fn Target(&self) -> [f32; 3] {
        return self._Target.to_array();
    }
    pub fn Orientation(&self) -> [f32; 4] {
        return self._Orientation.to_array();
    }
    /// Computes the scale from view coordinates (pixels) to world coordinates.
    pub fn ViewScale(&self, viewport_height: f32, scene_bounds: ([f32; 3], [f32; 3])) -> f32 {
        return self.ViewScaleWithThreshold(viewport_height, scene_bounds, ViewBox::DEFAULT_MIN_PIXELS);
    }

    /// Computes view-to-world scale taking depth into account:
    /// if the scale along any depth (such as the far depth of the scene) takes the worldview scale
    /// below the pixel threshold, that depth can be uniformly heuristically assumed to hold for all.
    pub fn ViewScaleWithThreshold(
        &self,
        viewport_height: f32,
        scene_bounds: ([f32; 3], [f32; 3]),
        min_pixel_threshold: f32,
    ) -> f32 {
        let extent = [
            scene_bounds.1[0] - scene_bounds.0[0],
            scene_bounds.1[1] - scene_bounds.0[1],
            scene_bounds.1[2] - scene_bounds.0[2],
        ];
        let radius = (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 0.5;
        let scale_to_source = if radius > 0.0 { radius } else { 1.0 };
        let fov_factor = (Self::FOV * 0.5).tan();
        let vp_h = viewport_height.max(1.0);

        let d_target = self._Distance;
        let d_far = self._Distance + radius;

        // View-to-world scale along far depth:
        let v2w_far = (2.0 * d_far * fov_factor / vp_h) * scale_to_source;
        let w2v_far = if v2w_far > 0.0 { 1.0 / v2w_far } else { 0.0 };

        // If the scale along far depth takes the worldview scale below the pixel threshold,
        // that depth is uniformly heuristically assumed to hold for all.
        let effective_depth = if w2v_far < min_pixel_threshold && d_far > 0.0 {
            d_far
        } else {
            d_target
        };

        let height_norm = 2.0 * effective_depth * fov_factor;
        return (height_norm / vp_h) * scale_to_source;
    }

    pub fn ViewBox(
        &self,
        viewport_pixels: [f32; 2],
        scene_bounds: ([f32; 3], [f32; 3]),
    ) -> ViewBox {
        return self.ViewBoxWithThreshold(
            viewport_pixels,
            scene_bounds,
            ViewBox::DEFAULT_MIN_PIXELS,
            ViewBox::DEFAULT_GPU_MEMORY_THRESHOLD_BYTES,
        );
    }

    pub fn ViewBoxWithThreshold(
        &self,
        viewport_pixels: [f32; 2],
        scene_bounds: ([f32; 3], [f32; 3]),
        min_pixel_threshold: f32,
        gpu_memory_threshold: u64,
    ) -> ViewBox {
        let scale_v2w = self.ViewScaleWithThreshold(viewport_pixels[1], scene_bounds, min_pixel_threshold);
        let extent = [
            scene_bounds.1[0] - scene_bounds.0[0],
            scene_bounds.1[1] - scene_bounds.0[1],
            scene_bounds.1[2] - scene_bounds.0[2],
        ];
        let center = [
            scene_bounds.0[0] + extent[0] * 0.5,
            scene_bounds.0[1] + extent[1] * 0.5,
            scene_bounds.0[2] + extent[2] * 0.5,
        ];
        let radius = (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 0.5;
        let scale_to_source = if radius > 0.0 { radius } else { 1.0 };
        let target = self._Target;
        let world_offset = [
            center[0] + target.x * scale_to_source,
            center[1] + target.y * scale_to_source,
            center[2] + target.z * scale_to_source,
        ];
        return ViewBox::New(viewport_pixels, world_offset, scale_v2w)
            .WithMinPixelThreshold(min_pixel_threshold)
            .WithGpuMemoryThreshold(gpu_memory_threshold);
    }
}

//-------------------------------------------------------------------------------------------------
