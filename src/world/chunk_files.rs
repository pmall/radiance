//! Where and how the chunk and module files are stored, and the catalog loaded from them. Only
//! this module knows the layout of `assets/`: the shared modules in `modules/`, and a folder per
//! biome in `biomes/<name>/` with its optional `biome.ron`, its `chunks/` (one `Chunk` per file)
//! and its `modules/` (a list of `Module`s per file), all written in RON. It reads them, keeps the
//! catalogs `chunks.rs` assembles from them, and reloads them when a file changes (`poll`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::SystemTime;

use serde::Deserialize;

use super::chunks::{Catalogs, Chunk, Module};

const ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");

/// A biome's own file (`assets/biomes/<name>/biome.ron`, optional).
#[derive(Deserialize, Default)]
struct BiomeFile {
    /// Biomes whose chunks and modules this one also draws from (its own win on a name).
    #[serde(default)]
    include: Vec<String>,
}

/// A biome's own files, read.
pub struct BiomeFiles {
    pub name: String,
    /// Biomes whose chunks and modules this one also draws from (its own win on a name).
    pub include: Vec<String>,
    pub chunks: HashMap<String, Chunk>,
    pub modules: HashMap<String, Module>,
}

/// Everything an assets folder holds, read.
pub struct Files {
    /// The modules every biome can use.
    pub shared: HashMap<String, Module>,
    /// Each biome's own files, by folder name, sorted.
    pub biomes: Vec<BiomeFiles>,
    /// Files that did not load, with the reason.
    pub errors: Vec<String>,
}

/// RON as chunk files are written: optional values without `Some(...)` (`accent: (150, 52, 40)`).
fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

fn ron_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ron"))
        .collect();
    files.sort();
    files
}

/// Biome folders under `<root>/biomes/`, sorted.
fn biome_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("biomes"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn read<T: serde::de::DeserializeOwned>(path: &Path, errors: &mut Vec<String>) -> Option<T> {
    match std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|s| ron_options().from_str::<T>(&s).map_err(|e| e.to_string()))
    {
        Ok(v) => Some(v),
        Err(e) => {
            errors.push(format!("{}: {e}", path.display()));
            None
        }
    }
}

fn read_modules(dir: &Path, errors: &mut Vec<String>) -> HashMap<String, Module> {
    ron_files(dir)
        .iter()
        .filter_map(|p| read::<Vec<Module>>(p, errors))
        .flatten()
        .map(|m| (m.name.clone(), m))
        .collect()
}

/// Reads the files of an assets folder (`assets/`, or a test's own).
fn read_all(root: &Path) -> Files {
    let mut errors = Vec::new();
    let shared = read_modules(&root.join("modules"), &mut errors);
    let mut biomes = Vec::new();
    for dir in biome_dirs(root) {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let file = dir.join("biome.ron");
        let include = if file.exists() {
            read::<BiomeFile>(&file, &mut errors)
                .unwrap_or_default()
                .include
        } else {
            Vec::new()
        };
        let chunks = ron_files(&dir.join("chunks"))
            .iter()
            .filter_map(|p| read::<Chunk>(p, &mut errors))
            .map(|c| (c.name.clone(), c))
            .collect();
        let modules = read_modules(&dir.join("modules"), &mut errors);
        biomes.push(BiomeFiles {
            name,
            include,
            chunks,
            modules,
        });
    }
    Files {
        shared,
        biomes,
        errors,
    }
}

/// The catalogs of an assets folder.
fn load_from(root: &Path) -> Catalogs {
    Catalogs::assemble(read_all(root))
}

fn load() -> Catalogs {
    load_from(Path::new(ASSETS))
}

fn store() -> &'static RwLock<Arc<Catalogs>> {
    static STORE: OnceLock<RwLock<Arc<Catalogs>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(Arc::new(load())))
}

/// Every biome's catalog, as currently loaded.
pub fn catalogs() -> Arc<Catalogs> {
    store().read().unwrap().clone()
}

/// Newest modification time of the catalog's files.
fn stamp() -> Option<SystemTime> {
    let root = Path::new(ASSETS);
    let mut dirs = vec![root.join("modules")];
    for d in biome_dirs(root) {
        dirs.extend([d.join("chunks"), d.join("modules"), d.clone()]);
    }
    dirs.iter()
        .flat_map(|d| ron_files(d))
        .filter_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()
}

/// Reloads the catalog if a file changed on disk. Returns true when it did (the world must then be
/// generated again).
pub fn poll() -> bool {
    static LAST: RwLock<Option<SystemTime>> = RwLock::new(None);
    let now = stamp();
    let mut last = LAST.write().unwrap();
    if last.is_none() {
        *last = now;
        return false;
    }
    if *last == now {
        return false;
    }
    *last = now;
    *store().write().unwrap() = Arc::new(load());
    GENERATION.fetch_add(1, Ordering::Relaxed);
    true
}

static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Bumped on every reload, so caches built from the catalog know when to start over.
pub fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biomes_include_other_biomes() {
        let root = std::env::temp_dir().join(format!("radiance-biomes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |path: &str, text: &str| {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        let chunk = |name: &str| {
            format!("Chunk(name: \"{name}\", size: (1, 1), parts: [Module(name: \"post\")])")
        };
        write("modules/a.ron", "[Module(name: \"post\", parts: [])]");
        write("biomes/default/chunks/house.ron", &chunk("house"));
        write("biomes/rust/biome.ron", "(include: [\"default\"])");
        write("biomes/rust/chunks/scrap.ron", &chunk("scrap"));
        write(
            "biomes/rust/modules/m.ron",
            "[Module(name: \"post\", parts: [Box(at: (0.0, 0.0, 0.0), size: (1.0, 1.0, 1.0), mat: Rust)])]",
        );
        let all = load_from(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert!(all.errors.is_empty(), "{:?}", all.errors);
        let (d, r) = (&all.biomes["default"], &all.biomes["rust"]);
        assert!(d.chunks.contains_key("house") && !d.chunks.contains_key("scrap"));
        assert!(r.chunks.contains_key("house") && r.chunks.contains_key("scrap"));
        // The biome's own module wins over the shared one of the same name.
        assert!(d.modules["post"].parts.is_empty());
        assert_eq!(r.modules["post"].parts.len(), 1);
    }
}
