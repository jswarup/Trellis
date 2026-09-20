// camera.rs ------------------------------------------------------------------------------------------------------------------

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
}

//-------------------------------------------------------------------------------------------------
