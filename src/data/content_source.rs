//! Game-owned locations for runtime campaign and scenario overrides.

pub fn candidate_dirs(rel: &str) -> Vec<std::path::PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut dirs = vec![std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                dirs.push(parent.join(rel));
            }
        }
        dirs
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = rel;
        Vec::new()
    }
}
