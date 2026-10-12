pub mod config;
pub mod manifest;
pub mod registry;
pub mod scaffold;

pub use config::GlobalConfig;
pub use manifest::{FastManifest, PackageConfig, ProjectType};
pub use registry::{resolve_and_fetch_dependency, DependencyRegistry, KnownDependency};
pub use scaffold::{create_new_project, init_project_in_dir};
