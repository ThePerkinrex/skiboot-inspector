use thiserror::Error;
use tracing::info;

use crate::app::resources::Loader;

pub struct StaticLoader {}

impl StaticLoader {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for StaticLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
#[error("File not found: {0:?}")]
pub struct FileNotFoundError(String);

impl Loader for StaticLoader {
    type Error = FileNotFoundError;

    type Reader<'a> = &'static [u8];

    async fn get_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<Self::Reader<'_>, Self::Error> {
        let p = path.as_ref().display().to_string();
        info!("Loading {p}");
        match &*p {
            "boot_lala.mtl" => Ok(include_bytes!("static/boot_lala.mtl")),
            "boot_lala.obj" => Ok(include_bytes!("static/boot_lala.obj")),
            _ => Err(FileNotFoundError(p.clone())),
        }
    }
}
