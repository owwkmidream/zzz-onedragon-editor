use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    project_root: Mutex<Option<PathBuf>>,
}

impl AppState {
    pub fn set_project_root(&self, path: PathBuf) {
        let mut guard = self.project_root.lock().expect("project_root mutex poisoned");
        *guard = Some(path);
    }

    pub fn project_root(&self) -> Option<PathBuf> {
        let guard = self.project_root.lock().expect("project_root mutex poisoned");
        guard.clone()
    }
}
