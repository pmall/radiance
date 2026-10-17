//! Stylized rendering pipeline.
//!
//! 1. Scene pass: world mesh, cel-shaded, into an offscreen HDR target with normals,
//!    object ids and depth (`shaders/scene.*`).
//! 2. Post pass: sky, ink outlines, colored height fog, grading and paper grain
//!    (`shaders/post.fs`).
//!
//! Shaders hot-reload from disk when edited.

mod cover;
mod facades;
mod halos;
mod lights;
pub mod look;
mod mesh;
mod overlay;
mod particles;
mod pixels;
mod props;
mod shader;
mod shadow;
mod surfaces;
mod targets;

use glam::{Mat4, Vec2, Vec3, vec3};
use sola_raylib::ffi;

use crate::world::Light;
use crate::world::World;
use cover::CoverMap;
use lights::LightGrid;
use look::Sky;
use mesh::WorldMesh;
use overlay::Overlay;
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
pub const FX_TEXTURE: i32 = 128;
pub const FX_ALL: i32 = 255;

/// Effect bits with their names, for the debug overlay.
pub const EFFECTS: [(i32, &str); 8] = [
    (FX_OUTLINES, "outlines"),
    (FX_FOG, "fog"),
    (FX_CEL, "cel"),
    (FX_SHADOWS, "shadows"),
    (FX_GRADING, "grading"),
    (FX_GRAIN, "grain"),
    (FX_PARTICLES, "particles"),
    (FX_TEXTURE, "texture"),
];

pub const RENDER_SCALES: [f32; 4] = [0.5, 0.75, 1.0, 1.5];

const NEAR: f32 = 0.1;
const FAR: f32 = 1500.0;
/// Outline thickness in pixels at 1080p.
const LINE_WIDTH: f32 = 2.4;
/// Props are drawn this far (meters; height differences count more, see `props::Reach`), and
/// cast shadows this far.
const PROP_REACH: f32 = 110.0;
const PROP_SHADOW_REACH: f32 = 45.0;
/// The lots' small boxes (`mesh::is_detail`) are drawn within this horizontal distance of the
/// camera (to the lot's edge), and cast shadows within the second.
const DETAIL_REACH: f32 = 110.0;
const DETAIL_SHADOW_REACH: f32 = 45.0;
/// How far ahead of the camera the shadow map is centered, in meters.
const SHADOW_LEAD: f32 = 50.0;
const RELOAD_POLL: f32 = 0.5;
/// Scales how fast fog thins with height, so it spans the whole floor stack.
const FOG_STACK_SCALE: f32 = 0.4;

const IDENTITY: ffi::Matrix = matrix(Mat4::IDENTITY);

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
    /// Each lot's small boxes, drawn near the camera only.
    details: HashMap<(i32, i32), WorldMesh>,
    /// Props of each lot, drawn instanced (`props.rs`).
    lots_props: HashMap<(i32, i32), Vec<crate::world::props::Placed>>,
    prop_meshes: props::PropMeshes,
    /// Prop triangles drawn last frame.
    prop_triangles: i32,
    detail_triangles: i32,
    /// Each lot's lights, with their index in the lot (a soul's identity).
    lots_lights: HashMap<(i32, i32), Vec<(usize, Light)>>,
    /// Souls that are singing: their lights swell and fade back (1 = just started).
    pulses: HashMap<crate::audio::Soul, f32>,
    light_grid: LightGrid,
    cover: CoverMap,
    facades: facades::Facades,
    /// Seed of the synced world, for the cover map.
    seed: u64,
    particles: Overlay,
    halos: Overlay,
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

/// raylib's matrix, whose fields are named by column-major index.
const fn matrix(m: Mat4) -> ffi::Matrix {
    let c = m.to_cols_array();
    ffi::Matrix {
        m0: c[0],
        m1: c[1],
        m2: c[2],
        m3: c[3],
        m4: c[4],
        m5: c[5],
        m6: c[6],
        m7: c[7],
        m8: c[8],
        m9: c[9],
        m10: c[10],
        m11: c[11],
        m12: c[12],
        m13: c[13],
        m14: c[14],
        m15: c[15],
    }
}

/// The detail meshes of the lots within `reach` of `eye` (horizontally, to the lot's edge).
fn details_near(
    details: &HashMap<(i32, i32), WorldMesh>,
    eye: Vec3,
    reach: f32,
) -> impl Iterator<Item = &WorldMesh> {
    let lot = crate::world::city::LOT;
    details.iter().filter_map(move |(&(x, z), m)| {
        let (x0, z0) = (x as f32 * lot, z as f32 * lot);
        let dx = (x0 - eye.x).max(eye.x - x0 - lot).max(0.0);
        let dz = (z0 - eye.z).max(eye.z - z0 - lot).max(0.0);
        (dx * dx + dz * dz < reach * reach).then_some(m)
    })
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
        // Map slots 3 and 4 are the photographic detail textures, samplers `texture3`/`texture4`.
        for (slot, texture) in surfaces::SLOTS.into_iter().zip(surfaces::load()) {
            unsafe { (*material.maps.add(slot)).texture = texture };
        }
        let cover = CoverMap::new();
        unsafe { (*material.maps.add(cover::SLOT)).texture = cover.texture };
        let prop_meshes = props::PropMeshes::new();
        unsafe { (*material.maps.add(pixels::SLOT)).texture = pixels::load() };
        let facades = facades::Facades::new();
        unsafe { (*material.maps.add(facades::SLOT)).texture = facades.texture };
        surfaces::bind_samplers(&mut material);
        let halos = halos::new(light_grid.texture);
        Self {
            scene,
            post,
            material,
            shadow,
            targets: None,
            meshes: HashMap::new(),
            details: HashMap::new(),
            lots_props: HashMap::new(),
            prop_meshes,
            prop_triangles: 0,
            detail_triangles: 0,
            lots_lights: HashMap::new(),
            pulses: HashMap::new(),
            light_grid,
            cover,
            facades,
            seed: 0,
            particles: particles::new(),
            halos,
            lights_in_use: 0,
            effects: FX_ALL,
            render_scale: 1.0,
            reload_timer: 0.0,
        }
    }

    /// Makes the lights of souls that just started to sing swell.
    pub fn pulse(&mut self, souls: &[crate::audio::Soul]) {
        for s in souls {
            self.pulses.insert(*s, 1.0);
        }
    }

    pub fn clear_world(&mut self) {
        self.pulses.clear();
        self.meshes.clear();
        self.details.clear();
        self.lots_props.clear();
        self.lots_lights.clear();
    }

    /// Brings the per-lot meshes in line with the lots the world currently has loaded.
    pub fn sync_world(&mut self, world: &World) {
        self.seed = world.seed;
        if self.facades.refresh() {
            unsafe { (*self.material.maps.add(facades::SLOT)).texture = self.facades.texture };
        }
        let live: std::collections::HashSet<_> = world.lots().map(|(k, _)| k).collect();
        self.meshes.retain(|k, _| live.contains(k));
        self.details.retain(|k, _| live.contains(k));
        self.lots_props.retain(|k, _| live.contains(k));
        self.lots_lights.retain(|k, _| live.contains(k));
        for (k, lot) in world.lots() {
            if self.lots_props.contains_key(&k) {
                continue;
            }
            self.lots_props.insert(k, lot.props.clone());
            self.lots_lights
                .insert(k, lot.lights().map(|(i, l)| (i, *l)).collect());
            let key = crate::rng::hash_coords(world.seed, k.0, 0, k.1);
            if let Some(m) = WorldMesh::build(lot, key, false) {
                self.meshes.insert(k, m);
            }
            if let Some(m) = WorldMesh::build(lot, key, true) {
                self.details.insert(k, m);
            }
        }
    }

    /// Triangles drawn in the last frame.
    pub fn triangles(&self) -> i32 {
        self.meshes.values().map(|m| m.triangles()).sum::<i32>()
            + self.detail_triangles
            + self.prop_triangles
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
        self.halos.reload();
        self.material.shader = self.scene.raw;
        surfaces::bind_samplers(&mut self.material);
    }

    /// Polls shader files for changes.
    pub fn update(&mut self, dt: f32) {
        self.pulses.retain(|_, b| {
            *b -= dt / 3.5;
            *b > 0.0
        });
        self.reload_timer -= dt;
        if self.reload_timer <= 0.0 {
            self.reload_timer = RELOAD_POLL;
            self.scene.poll();
            self.post.poll();
            self.shadow.poll();
            self.particles.poll();
            self.halos.poll();
            self.material.shader = self.scene.raw;
            surfaces::bind_samplers(&mut self.material);
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

        let lots_props = || self.lots_props.values().map(|v| v.as_slice());
        let batches = self.prop_meshes.batches(
            lots_props(),
            &props::Reach {
                eye: view.pos,
                reach: PROP_REACH,
                ahead: Some(view.dir),
            },
        );
        self.prop_triangles = props::triangles(&batches);

        // Shadow pass.
        let shadows = self.effects & FX_SHADOWS != 0 && look.light.length_squared() > 0.0;
        let mut light_vp = glam::Mat4::ZERO;
        if shadows {
            let ahead = vec3(view.dir.x, 0.0, view.dir.z).normalize_or_zero() * SHADOW_LEAD;
            // Small props' shadows only show up close.
            let casters = self.prop_meshes.batches(
                lots_props(),
                &props::Reach {
                    eye: view.pos,
                    reach: PROP_SHADOW_REACH,
                    ahead: None,
                },
            );
            let near = details_near(&self.details, view.pos, DETAIL_SHADOW_REACH);
            light_vp = self.shadow.render(
                self.meshes.values().chain(near).map(|m| &m.raw),
                &casters,
                view.pos + ahead,
                sky.light_dir,
            );
        }

        // Scene pass.
        self.cover.update(self.seed, view.pos);
        let cover_origin = self.cover.origin_uniform();
        let s = &mut self.scene;
        s.set_vec2("uCoverOrigin", cover_origin);
        s.set_f32_array("uFloorY", &cover::floor_uniform());
        s.set_i32("uFloorCount", crate::world::floors::count() as i32);
        s.set_mat4("uLightVP", light_vp);
        s.set_i32("uShadows", shadows as i32);
        s.set_f32("uShadowTexel", self.shadow.texel());
        s.set_vec3_array("uPalette", &crate::world::biome::palette_uniform());
        s.set_vec3("uLightDir", sky.light_dir);
        s.set_vec3("uLight", look.light);
        s.set_vec3("uAmbientSky", look.ambient_sky);
        s.set_vec3("uAmbientGround", look.ambient_ground);
        s.set_i32("uEffects", self.effects);
        // A singing soul's light swells and fades over a few seconds.
        let pulses = &self.pulses;
        let lights = self.lots_lights.iter().flat_map(|(key, ls)| {
            ls.iter().map(move |&(index, ref l)| {
                let b = pulses
                    .get(&crate::audio::Soul { lot: *key, index })
                    .copied()
                    .unwrap_or(0.0);
                Light {
                    color: l.color * (1.0 + 1.8 * b),
                    radius: l.radius * (1.0 + 0.35 * b),
                    ..*l
                }
            })
        });
        self.lights_in_use = self.light_grid.update(lights, view.pos);
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
            s.set_i32("uInstanced", 0);
            let mut detail = 0;
            for mesh in self.meshes.values() {
                ffi::DrawMesh(mesh.raw, self.material, IDENTITY);
            }
            for mesh in details_near(&self.details, view.pos, DETAIL_REACH) {
                ffi::DrawMesh(mesh.raw, self.material, IDENTITY);
                detail += mesh.triangles();
            }
            self.detail_triangles = detail;
            s.set_i32("uInstanced", 1);
            props::draw(&batches, self.material);
            s.set_i32("uInstanced", 0);
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
        p.set_vec2("uCoverOrigin", cover_origin);
        p.set_f32_array("uFloorY", &cover::floor_uniform());
        p.set_i32("uFloorCount", crate::world::floors::count() as i32);
        p.set_f32(
            "uFogBase",
            crate::world::floors::y(crate::world::floors::START),
        );
        p.set_f32("uFogDensity", look.fog_density);
        // The looks were tuned for a city a third as tall: stretch the thinning over the stack.
        p.set_f32("uFogFalloff", look.fog_falloff * FOG_STACK_SCALE);
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
            p.set_texture("uCover", self.cover.texture);
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

        // Particles, then the halos at the lights.
        if self.effects & FX_PARTICLES != 0 {
            // Spores rather than pollen where no sky is overhead.
            let depth_k = 1.0 - self.cover.sky_view(view.pos);
            // Pollen is warm; it picks up a little of the sun or moon color.
            let mote = look
                .disk
                .clamp(Vec3::ZERO, Vec3::ONE)
                .powf(1.0 / 2.2)
                .lerp(vec3(1.0, 0.78, 0.4), 0.6);
            let p = &mut self.particles.shader;
            p.set_f32("uTime", time);
            p.set_f32("uDepthK", depth_k);
            p.set_vec3("uMote", mote);
            self.halos
                .shader
                .set_i32("uCount", self.lights_in_use as i32);
            let (cam, glow) = ((right, cam_up), look.plant_glow);
            for layer in [&mut self.particles, &mut self.halos] {
                layer.draw(view, cam, &camera, (rw, rh), targets.depth, glow);
            }
        }
    }
}
