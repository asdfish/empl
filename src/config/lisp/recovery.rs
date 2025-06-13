use std::{
    borrow::Cow,
    fs::{DirBuilder, File},
    io::{self, Write},
    path::Path,
};

pub fn create_default_config<P>(path: &P) -> Result<Cow<'static, str>, io::Error>
where
    P: AsRef<Path>,
{
    const DEFAULT_CONFIG: &str = include_str!("../../../main.lisp");

    if let Err(err) = path
        .as_ref()
        .parent()
        .map(|parent| DirBuilder::new().recursive(true).create(parent))
        .transpose()
    {
        match err.kind() {
            io::ErrorKind::AlreadyExists => {}
            _ => return Err(err),
        }
    }

    File::create(path)
        .and_then(|mut file| {
            file.write_all(DEFAULT_CONFIG.as_bytes())
                .and_then(|_| file.flush())
        })
        .map(|_| DEFAULT_CONFIG)
        .map(Cow::Borrowed)
}
