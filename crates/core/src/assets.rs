// Asset sources for `<Image src="asset://..."/>` lookups.
//
// `EmbeddedAssets` — the full dist/ tree baked in at compile time via
// `include_dir!`; used by `BundleSource::EmbeddedDir`.
// `DiskAssets` — reads from dist/ on disk each load; debug-only, for dev mode
// (`GLUXE_DEV_DIST`). Hashed filenames mean stale assets arrive under new
// paths, so GPUI-side caches are never served stale bytes.
//
// Load paths have leading "./" / "/" stripped so bare and slash-prefixed paths
// resolve to the same key.

use std::borrow::Cow;
#[cfg(debug_assertions)]
use std::path::{Path, PathBuf};

use gpui::{AssetSource, Result, SharedString};
use include_dir::Dir;

fn normalize(path: &str) -> &str {
    path.trim_start_matches("./").trim_start_matches('/')
}

pub struct EmbeddedAssets {
    dir: &'static Dir<'static>,
}

impl EmbeddedAssets {
    pub fn new(dir: &'static Dir<'static>) -> Self {
        Self { dir }
    }
}

impl AssetSource for EmbeddedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let key = normalize(path);
        Ok(self.dir.get_file(key).map(|f| Cow::Borrowed(f.contents())))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let prefix = normalize(path);

        fn walk(dir: &'static Dir<'static>, out: &mut Vec<SharedString>) {
            for f in dir.files() {
                out.push(f.path().to_string_lossy().to_string().into());
            }
            for d in dir.dirs() {
                walk(d, out);
            }
        }

        let mut out = Vec::new();
        match (prefix.is_empty(), self.dir.get_dir(prefix)) {
            (true, _) => walk(self.dir, &mut out),     // whole tree
            (false, Some(sub)) => walk(sub, &mut out), // real subdir (paths stay dist-root-relative)
            (false, None) => {
                // partial path/stem — filtered walk
                walk(self.dir, &mut out);
                out.retain(|p| p.starts_with(prefix));
            }
        }
        Ok(out)
    }
}

/// Dev-mode only; release builds always use the embedded path.
#[cfg(debug_assertions)]
pub struct DiskAssets {
    root: PathBuf,
}

#[cfg(debug_assertions)]
impl DiskAssets {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[cfg(debug_assertions)]
impl AssetSource for DiskAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let key = normalize(path);
        match std::fs::read(self.root.join(key)) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            Err(_) => Ok(None),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let prefix = normalize(path);

        // Use `/` separators to match `asset://` URLs — Windows yields `\` otherwise.
        fn walk(root: &Path, dir: &Path, out: &mut Vec<SharedString>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(root, &p, out);
                } else if let Ok(rel) = p.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/").into());
                }
            }
        }

        let mut out = Vec::new();
        let sub = self.root.join(prefix);
        if !prefix.is_empty() && sub.is_dir() {
            walk(&self.root, &sub, &mut out);
        } else {
            walk(&self.root, &self.root, &mut out);
            if !prefix.is_empty() {
                out.retain(|p| p.starts_with(prefix));
            }
        }
        Ok(out)
    }
}

/// Either asset backend as a single concrete type for `application().with_assets(..)`.
pub(crate) enum RuntimeAssets {
    Embedded(EmbeddedAssets),
    #[cfg(debug_assertions)]
    Disk(DiskAssets),
}

impl AssetSource for RuntimeAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match self {
            RuntimeAssets::Embedded(a) => a.load(path),
            #[cfg(debug_assertions)]
            RuntimeAssets::Disk(a) => a.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        match self {
            RuntimeAssets::Embedded(a) => a.list(path),
            #[cfg(debug_assertions)]
            RuntimeAssets::Disk(a) => a.list(path),
        }
    }
}
