//! Shaders loaded from `shaders/` and hot-reloaded when the files change. The sources
//! are also embedded at build time as a fallback when the directory is missing.
//!
//! GLSL has no includes: a line `#include "file"` is replaced here by a file of `INCLUDES`, and a
//! change to that file reloads the shaders using it. Compile errors in an included file are
//! reported as source string 1, with the file's own line numbers.

use std::collections::HashMap;
use std::ffi::CString;
use std::path::PathBuf;
use std::time::SystemTime;

use glam::{Mat4, Vec2, Vec3};
use sola_raylib::ffi;

const SHADER_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders");

/// The files shaders may include: name and embedded source.
const INCLUDES: &[(&str, &str)] = &[
    ("cover.glsl", include_str!("../../shaders/cover.glsl")),
    ("fade.glsl", include_str!("../../shaders/fade.glsl")),
];

pub struct HotShader {
    pub raw: ffi::Shader,
    vs: Option<(&'static str, &'static str)>,
    fs: (&'static str, &'static str),
    stamp: Option<SystemTime>,
    /// The files of `INCLUDES` the sources included when last loaded.
    includes: Vec<&'static str>,
    locs: HashMap<&'static str, i32>,
    /// Last load failed; the previous working version is still in use.
    pub broken: bool,
}

/// A file of `shaders/`: from disk if `disk` and it is there, else as embedded.
fn read(name: &str, embedded: &'static str, disk: bool) -> String {
    let file = disk.then(|| std::fs::read_to_string(PathBuf::from(SHADER_DIR).join(name)));
    file.and_then(Result::ok)
        .unwrap_or_else(|| embedded.to_owned())
}

/// A shader's source with its `#include` lines replaced, and the files included.
fn source(name: &str, embedded: &'static str, disk: bool) -> (String, Vec<&'static str>) {
    let mut out = String::new();
    let mut included = Vec::new();
    for (i, line) in read(name, embedded, disk).lines().enumerate() {
        let file = line
            .trim()
            .strip_prefix("#include")
            .map(|f| f.trim().trim_matches('"'));
        // An unknown file keeps its line, which then fails to compile.
        let Some(&(file, text)) = file.and_then(|f| INCLUDES.iter().find(|(n, _)| *n == f)) else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        // The next line of the shader is line i + 2 of source string 0.
        out.push_str(&format!(
            "#line 1 1\n{}\n#line {} 0\n",
            read(file, text, disk),
            i + 2
        ));
        included.push(file);
    }
    (out, included)
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
            includes: Vec::new(),
            locs: HashMap::new(),
            broken: false,
        };
        // A file broken on disk at startup: the embedded version runs until the file is fixed.
        if !s.load(true) {
            if !s.load(false) {
                panic!("built-in shader {} failed to compile", fs.0);
            }
            s.broken = true;
        }
        s.stamp = s.newest_stamp();
        s
    }

    fn newest_stamp(&self) -> Option<SystemTime> {
        let own = self.vs.map(|(n, _)| n).into_iter().chain([self.fs.0]);
        own.chain(self.includes.iter().copied())
            .filter_map(modified)
            .max()
    }

    fn load(&mut self, disk: bool) -> bool {
        let (fs, mut includes) = source(self.fs.0, self.fs.1, disk);
        let vs = self.vs.map(|(n, e)| {
            let (vs, more) = source(n, e, disk);
            includes.extend(more);
            CString::new(vs).unwrap()
        });
        self.includes = includes;
        let fs = CString::new(fs).unwrap();
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

    /// Reloads if a source file changed on disk.
    pub fn poll(&mut self) {
        let stamp = self.newest_stamp();
        if stamp != self.stamp {
            self.stamp = stamp;
            self.load(true);
        }
    }

    pub fn force_reload(&mut self) {
        self.load(true);
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

    /// Sets a `float[]` uniform.
    pub fn set_f32_array(&mut self, name: &'static str, data: &[f32]) {
        let loc = self.loc(name);
        if loc >= 0 {
            unsafe {
                ffi::SetShaderValueV(
                    self.raw,
                    loc,
                    data.as_ptr() as _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32,
                    data.len() as i32,
                )
            };
        }
    }

    /// Sets an `int[]` uniform.
    pub fn set_i32_array(&mut self, name: &'static str, data: &[i32]) {
        let loc = self.loc(name);
        if loc >= 0 {
            unsafe {
                ffi::SetShaderValueV(
                    self.raw,
                    loc,
                    data.as_ptr() as _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_INT as i32,
                    data.len() as i32,
                )
            };
        }
    }

    /// Sets a `vec3[]` uniform from packed floats (3 per element).
    pub fn set_vec3_array(&mut self, name: &'static str, data: &[f32]) {
        let loc = self.loc(name);
        if loc >= 0 {
            unsafe {
                ffi::SetShaderValueV(
                    self.raw,
                    loc,
                    data.as_ptr() as _,
                    ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC3 as i32,
                    (data.len() / 3) as i32,
                )
            };
        }
    }

    pub fn set_mat4(&mut self, name: &'static str, m: Mat4) {
        let loc = self.loc(name);
        if loc >= 0 {
            unsafe { ffi::SetShaderValueMatrix(self.raw, loc, super::matrix(m)) };
        }
    }

    pub fn set_texture(&mut self, name: &'static str, tex: ffi::Texture2D) {
        let loc = self.loc(name);
        if loc >= 0 {
            unsafe { ffi::SetShaderValueTexture(self.raw, loc, tex) };
        }
    }
}
