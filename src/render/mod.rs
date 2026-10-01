//! Stylized rendering pipeline.
//!
//! 1. Scene pass: world mesh, cel-shaded, into an offscreen HDR target with normals,
//!    object ids and depth (`shaders/scene.*`).
//! 2. Post pass: sky, ink outlines, colored height fog, grading and paper grain
//!    (`shaders/post.fs`).
//!
//! Shaders hot-reload from disk when edited.

mod lights;
pub mod look;
mod mesh;
mod particles;
mod shader;
mod shadow;
mod targets;

use glam::{Vec2, Vec3, vec3};
use sola_raylib::ffi;

use crate::world::Light;
use crate::world::World;
use lights::LightGrid;
use look::Sky;
use mesh::WorldMesh;
use particles::Particles;
use shader::HotShader;
use shadow::ShadowMap;
use std::collections::HashMap;
use targets::SceneTargets;

pub const FX_OUTLINES: i32 = 1;
pub const FX_FOG: i32 = 2;
pub const FX_CEL: i32 = 4;
pub const FX_GRADING: i32 = 8;
pub const FX_GRAIN: i32 = 16;
pub const FX_SHADOWS: i32 = 32;
pub const FX_PARTICLES: i32 = 64;
pub const FX_ALL: i32 = 127;

/// Effect bits with their names, for the debug overlay.
pub const EFFECTS: [(i32, &str); 7] = [
    (FX_OUTLINES, "outlines"),
    (FX_FOG, "fog"),
    (FX_CEL, "cel"),
    (FX_SHADOWS, "shadows"),
    (FX_GRADING, "grading"),
    (FX_GRAIN, "grain"),
    (FX_PARTICLES, "particles"),
];

pub const RENDER_SCALES: [f32; 4] = [0.5, 0.75, 1.0, 1.5];

const NEAR: f32 = 0.1;
const FAR: f32 = 1500.0;
/// Outline thickness in pixels at 1080p.
const LINE_WIDTH: f32 = 2.4;
/// How far ahead of the camera the shadow map is centered, in meters.
const SHADOW_LEAD: f32 = 50.0;
const RELOAD_POLL: f32 = 0.5;

const IDENTITY: ffi::Matrix = ffi::Matrix {
    m0: 1.0,
    m4: 0.0,
    m8: 0.0,
    m12: 0.0,
    m1: 0.0,
    m5: 1.0,
    m9: 0.0,
    m13: 0.0,
    m2: 0.0,
    m6: 0.0,
    m10: 1.0,
    m14: 0.0,
    m3: 0.0,
    m7: 0.0,
    m11: 0.0,
    m15: 1.0,
};

pub struct View {
    pub pos: Vec3,
    pub dir: Vec3,
    /// Vertical field of view in degrees.
    pub fovy: f32,
}

pub struct Renderer {
    scene: HotShader,
    post: HotShader,
    material: ffi::Material,
    shadow: ShadowMap,
    targets: Option<SceneTargets>,
    meshes: HashMap<(i32, i32), WorldMesh>,
    lots_lights: HashMap<(i32, i32), Vec<Light>>,
    light_grid: LightGrid,
    particles: Particles,
    pub lights_in_use: usize,
    pub effects: i32,
    pub render_scale: f32,
    reload_timer: f32,
}

fn v3(v: Vec3) -> ffi::Vector3 {
    ffi::Vector3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

impl Renderer {
    /// Requires an open window (GL context).
    pub fn new() -> Self {
        let scene = HotShader::new(
            Some(("scene.vs", include_str!("../../shaders/scene.vs"))),
            ("scene.fs", include_str!("../../shaders/scene.fs")),
        );
        let post = HotShader::new(None, ("post.fs", include_str!("../../shaders/post.fs")));
        let shadow = ShadowMap::new();
        let mut material = unsafe { ffi::LoadMaterialDefault() };
        material.shader = scene.raw;
        // Map slot 1 is bound by DrawMesh to the `texture1` sampler.
        unsafe { (*material.maps.add(1)).texture = shadow.depth };
        // Map slot 2 is bound to `texture2`: the plant light grid.
        let light_grid = LightGrid::new();
        unsafe { (*material.maps.add(2)).texture = light_grid.texture };
        Self {
            scene,
            post,
            material,
            shadow,
            targets: None,
            meshes: HashMap::new(),
            lots_lights: HashMap::new(),
            light_grid,
            particles: Particles::new(),
            lights_in_use: 0,
            effects: FX_ALL,
            render_scale: 1.0,
            reload_timer: 0.0,
        }
    }

    pub fn clear_world(&mut self) {
        self.meshes.clear();
        self.lots_lights.clear();
    }

    /// Brings the per-lot meshes in line with the lots the world currently has loaded.
    pub fn sync_world(&mut self, world: &World) {
        let live: std::collections::HashSet<_> = world.lots().map(|(k, _)| k).collect();
        self.meshes.retain(|k, _| live.contains(k));
        self.lots_lights.retain(|k, _| live.contains(k));
        for (k, lot) in world.lots() {
            self.lots_lights
                .entry(k)
                .or_insert_with(|| lot.lights.clone());
            self.meshes.entry(k).or_insert_with(|| {
                let key = crate::rng::hash_coords(world.seed, k.0, 0, k.1);
                WorldMesh::build(lot, key)
            });
        }
    }

    pub fn triangles(&self) -> i32 {
        self.meshes.values().map(|m| m.triangles()).sum()
    }

    pub fn shaders_broken(&self) -> bool {
        self.scene.broken || self.post.broken
    }

    pub fn reload_shaders(&mut self) {
        self.scene.force_reload();
        self.post.force_reload();
        self.shadow.shader.force_reload();
        self.shadow.poll();
        self.particles.reload();
        self.material.shader = self.scene.raw;
    }

    /// Polls shader files for changes.
    pub fn update(&mut self, dt: f32) {
        self.reload_timer -= dt;
        if self.reload_timer <= 0.0 {
            self.reload_timer = RELOAD_POLL;
            self.scene.poll();
            self.post.poll();
            self.shadow.poll();
            self.particles.poll();
            self.material.shader = self.scene.raw;
        }
    }

    pub fn cycle_render_scale(&mut self) {
        let i = RENDER_SCALES
            .iter()
            .position(|s| *s == self.render_scale)
            .unwrap_or(0);
        self.render_scale = RENDER_SCALES[(i + 1) % RENDER_SCALES.len()];
    }

    /// Draws the world to the screen. Call between BeginDrawing and EndDrawing.
    pub fn draw(&mut self, view: &View, sky: &Sky, time: f32) {
        let (rw, rh) = unsafe { (ffi::GetRenderWidth(), ffi::GetRenderHeight()) };
        let sw = ((rw as f32 * self.render_scale) as i32).max(1);
        let sh = ((rh as f32 * self.render_scale) as i32).max(1);
        if self
            .targets
            .as_ref()
            .is_none_or(|t| t.width != sw || t.height != sh)
        {
            self.targets = None;
            self.targets = Some(SceneTargets::new(sw, sh));
        }
        let targets = self.targets.as_ref().unwrap();
        let look = &sky.look;

        let up = Vec3::Y;
        let right = view.dir.cross(up).normalize();
        let cam_up = right.cross(view.dir);
        let camera = ffi::Camera3D {
            position: v3(view.pos),
            target: v3(view.pos + view.dir),
            up: v3(up),
            fovy: view.fovy,
            projection: ffi::CameraProjection::CAMERA_PERSPECTIVE as i32,
        };

        // Shadow pass.
        let shadows = self.effects & FX_SHADOWS != 0 && look.light.length_squared() > 0.0;
        let mut light_vp = glam::Mat4::ZERO;
        if shadows {
            let ahead = vec3(view.dir.x, 0.0, view.dir.z).normalize_or_zero() * SHADOW_LEAD;
            light_vp = self.shadow.render(
                self.meshes.values().map(|m| &m.raw),
                view.pos + ahead,
                sky.light_dir,
            );
        }

        // Scene pass.
        let s = &mut self.scene;
        s.set_mat4("uLightVP", light_vp);
        s.set_i32("uShadows", shadows as i32);
        s.set_f32("uShadowTexel", self.shadow.texel());
        s.set_vec3("uLightDir", sky.light_dir);
        s.set_vec3("uLight", look.light);
        s.set_vec3("uAmbientSky", look.ambient_sky);
        s.set_vec3("uAmbientGround", look.ambient_ground);
        s.set_i32("uEffects", self.effects);
        self.lights_in_use = self
            .light_grid
            .update(self.lots_lights.values().flatten(), view.pos);
        let s = &mut self.scene;
        s.set_vec3("uGridOrigin", self.light_grid.origin);
        s.set_f32("uGlow", look.plant_glow);
        unsafe {
            ffi::rlSetClipPlanes(NEAR as f64, FAR as f64);
            ffi::BeginTextureMode(targets.render_texture());
            ffi::rlClearColor(0, 0, 0, 0);
            ffi::rlClearScreenBuffers();
            ffi::BeginMode3D(camera);
            // Alpha carries the object id in the normal target: it must not blend.
            ffi::rlDisableColorBlend();
            for mesh in self.meshes.values() {
                ffi::DrawMesh(mesh.raw, self.material, IDENTITY);
            }
            ffi::rlEnableColorBlend();
            ffi::EndMode3D();
            ffi::EndTextureMode();
        }

        // Post pass.
        let p = &mut self.post;
        p.set_vec2("uScreen", Vec2::new(rw as f32, rh as f32));
        p.set_vec2("uTexel", Vec2::new(1.0 / sw as f32, 1.0 / sh as f32));
        p.set_f32("uNear", NEAR);
        p.set_f32("uFar", FAR);
        p.set_vec3("uCamPos", view.pos);
        p.set_vec3("uCamFwd", view.dir);
        p.set_vec3("uCamRight", right);
        p.set_vec3("uCamUp", cam_up);
        p.set_f32("uTanHalfFov", (view.fovy.to_radians() * 0.5).tan());
        p.set_f32("uAspect", sw as f32 / sh as f32);
        p.set_vec3("uSkyZenith", look.sky_zenith);
        p.set_vec3("uSkyHorizon", look.sky_horizon);
        p.set_vec3("uDisk", look.disk);
        p.set_f32("uStars", look.stars);
        p.set_vec3("uLightDir", sky.light_dir);
        p.set_vec3("uFog", look.fog);
        p.set_f32("uFogDensity", look.fog_density);
        p.set_f32("uFogFalloff", look.fog_falloff);
        p.set_f32("uFogGlow", look.fog_glow);
        p.set_vec3("uInk", look.ink);
        p.set_f32("uExposure", look.exposure);
        p.set_f32("uSaturation", look.saturation);
        p.set_f32("uContrast", look.contrast);
        p.set_vec3("uShadowTint", look.shadow_tint);
        p.set_vec3("uHighlightTint", look.highlight_tint);
        p.set_i32("uEffects", self.effects);
        p.set_f32("uTime", time);
        p.set_f32("uLineWidth", (LINE_WIDTH * sh as f32 / 1080.0).max(1.0));
        unsafe {
            ffi::BeginShaderMode(p.raw);
            // Samplers must be bound after BeginShaderMode, which flushes pending state.
            p.set_texture("uNormal", targets.normal);
            p.set_texture("uDepth", targets.depth);
            let (w, h) = (ffi::GetScreenWidth() as f32, ffi::GetScreenHeight() as f32);
            ffi::DrawTexturePro(
                targets.color,
                ffi::Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: sw as f32,
                    height: sh as f32,
                },
                ffi::Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: w,
                    height: h,
                },
                ffi::Vector2 { x: 0.0, y: 0.0 },
                0.0,
                ffi::Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            );
            ffi::EndShaderMode();
        }

        if self.effects & FX_PARTICLES != 0 {
            self.draw_particles(view, sky, &camera, targets.depth, (rw, rh), (right, cam_up));
        }
    }

    /// Additive particles over the finished frame, hidden by scene depth.
    fn draw_particles(
        &mut self,
        view: &View,
        sky: &Sky,
        camera: &ffi::Camera3D,
        depth: ffi::Texture2D,
        screen: (i32, i32),
        (right, cam_up): (Vec3, Vec3),
    ) {
        let look = &sky.look;
        let k = ((view.pos.y + 60.0) / 50.0).clamp(0.0, 1.0);
        let depth_k = 1.0 - k * k * (3.0 - 2.0 * k);
        // Pollen is warm; it picks up a little of the sun or moon color.
        let disk = look
            .disk
            .clamp(Vec3::ZERO, Vec3::ONE)
            .powf(1.0 / 2.2)
            .lerp(vec3(1.0, 0.78, 0.4), 0.6);
        let time = unsafe { ffi::GetTime() } as f32;
        let p = &mut self.particles.shader;
        p.set_vec3("uCamPos", view.pos);
        p.set_vec3("uCamFwd", view.dir);
        p.set_vec3("uCamRight", right);
        p.set_vec3("uCamUp", cam_up);
        p.set_f32("uTime", time);
        p.set_vec2("uScreen", Vec2::new(screen.0 as f32, screen.1 as f32));
        p.set_f32("uNear", NEAR);
        p.set_f32("uFar", FAR);
        p.set_f32("uGlow", look.plant_glow);
        p.set_f32("uDepthK", depth_k);
        p.set_vec3("uMote", disk);
        self.particles.material.shader = self.particles.shader.raw;
        unsafe {
            (*self.particles.material.maps.add(1)).texture = depth;
            ffi::rlSetClipPlanes(NEAR as f64, FAR as f64);
            ffi::BeginMode3D(*camera);
            self.particles.draw();
            ffi::EndMode3D();
        }
    }
}
