//! Shaders loaded from `shaders/` and hot-reloaded when the files change. The sources
//! are also embedded at build time as a fallback when the directory is missing.

use std::collections::HashMap;
use std::ffi::CString;
use std::path::PathBuf;
use std::time::SystemTime;

use glam::{Mat4, Vec2, Vec3};
use sola_raylib::ffi;

const SHADER_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders");

pub struct HotShader {
    pub raw: ffi::Shader,
    vs: Option<(&'static str, &'static str)>,
    fs: (&'static str, &'static str),
    stamp: Option<SystemTime>,
    locs: HashMap<&'static str, i32>,
    /// Last load failed; the previous working version is still in use.
    pub broken: bool,
}

fn read(name: &str, embedded: &'static str) -> String {
    std::fs::read_to_string(PathBuf::from(SHADER_DIR).join(name))
        .unwrap_or_else(|_| embedded.to_owned())
}

fn modified(name: &str) -> Option<SystemTime> {
    std::fs::metadata(PathBuf::from(SHADER_DIR).join(name))
        .and_then(|m| m.modified())
        .ok()
}

impl HotShader {
    /// `vs` / `fs` are (file name, embedded source). No `vs` uses raylib's default.
    pub fn new(vs: Option<(&'static str, &'static str)>, fs: (&'static str, &'static str)) -> Self {
        let mut s = Self {
            raw: unsafe { std::mem::zeroed() },
            vs,
            fs,
            stamp: None,
            locs: HashMap::new(),
            broken: false,
        };
        s.stamp = s.newest_stamp();
        if !s.load() {
            panic!("built-in shader {} failed to compile", fs.0);
        }
        s
    }

    fn newest_stamp(&self) -> Option<SystemTime> {
        let fs = modified(self.fs.0);
        let vs = self.vs.and_then(|(n, _)| modified(n));
        fs.max(vs)
    }

    fn load(&mut self) -> bool {
        let vs = self.vs.map(|(n, e)| CString::new(read(n, e)).unwrap());
        let fs = CString::new(read(self.fs.0, self.fs.1)).unwrap();
        let shader = unsafe {
            ffi::LoadShaderFromMemory(
                vs.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                fs.as_ptr(),
            )
        };
        // raylib falls back to its default shader when compilation fails.
        if shader.id == 0 || shader.id == unsafe { ffi::rlGetShaderIdDefault() } {
            self.broken = true;
            return false;
        }
        if self.raw.id != 0 {
            unsafe { ffi::UnloadShader(self.raw) };
        }
        self.raw = shader;
        self.locs.clear();
        self.broken = false;
        true
    }

    /// Reloads if a source file changed on disk. Returns true when a reload happened.
    pub fn poll(&mut self) -> bool {
        let stamp = self.newest_stamp();
        if stamp != self.stamp {
            self.stamp = stamp;
            return self.load();
        }
        false
    }

    pub fn force_reload(&mut self) -> bool {
        self.load()
    }

    pub fn loc(&mut self, name: &'static str) -> i32 {
        let raw = self.raw;
        *self.locs.entry(name).or_insert_with(|| {
            let c = CString::new(name).unwrap();
            unsafe { ffi::GetShaderLocation(raw, c.as_ptr()) }
        })
    }

    fn set_raw(
        &mut self,
        name: &'static str,
        ptr: *const std::ffi::c_void,
        ty: ffi::ShaderUniformDataType,
    ) {
        let loc = self.loc(name);
        if loc >= 0 {
            unsafe { ffi::SetShaderValue(self.raw, loc, ptr, ty as i32) };
        }
    }

    pub fn set_f32(&mut self, name: &'static str, v: f32) {
        self.set_raw(
            name,
            &v as *const f32 as _,
            ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT,
        );
    }

    pub fn set_i32(&mut self, name: &'static str, v: i32) {
        self.set_raw(
            name,
            &v as *const i32 as _,
            ffi::ShaderUniformDataType::SHADER_UNIFORM_INT,
        );
    }

    pub fn set_vec2(&mut self, name: &'static str, v: Vec2) {
        let a = v.to_array();
        self.set_raw(
            name,
            a.as_ptr() as _,
            ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC2,
        );
    }

    pub fn set_vec3(&mut self, name: &'static str, v: Vec3) {
        let a = v.to_array();
        self.set_raw(
            name,
            a.as_ptr() as _,
            ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC3,
        );
    }

    pub fn set_mat4(&mut self, name: &'static str, m: Mat4) {
        let loc = self.loc(name);
        if loc >= 0 {
            // raylib's Matrix fields are named by column-major index.
            let c = m.to_cols_array();
            let mat = ffi::Matrix {
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
            };
            unsafe { ffi::SetShaderValueMatrix(self.raw, loc, mat) };
        }
    }

    pub fn set_texture(&mut self, name: &'static str, tex: ffi::Texture2D) {
        let loc = self.loc(name);
        if loc >= 0 {
            unsafe { ffi::SetShaderValueTexture(self.raw, loc, tex) };
        }
    }
}
