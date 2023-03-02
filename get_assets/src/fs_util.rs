
use crate::DataDir;
use std::{
    io::{
        ErrorKind,
        Result,
    },
    path::{
        Path,
        PathBuf,
    },
};
use tokio::{
    fs::{
        File,
        metadata,
        rename,
        create_dir_all,
    },
    io::AsyncWriteExt,
};


pub async fn exists<P: AsRef<Path>>(path: P) -> Result<bool> {
    match metadata(path).await {
        Ok(_) => Ok(true),
        Err(e) =>
            if e.kind() == ErrorKind::NotFound { Ok(false) }
            else { Err(e) }
    }
}

pub async fn create_tmp_file(
    base: &DataDir,
    default_name: &str,
) -> Result<(File, PathBuf)> {
    let tmp_dir = base.tmp_subdir();
    create_dir_all(&tmp_dir).await?;

    let mut path = base.tmp_subdir().join(default_name);
    let mut i = 0;

    loop {
        match File::create(&path).await {
            Ok(file) => return Ok((file, path)),
            Err(e) =>
                if e.kind() == ErrorKind::AlreadyExists {
                    let name = format!("{}{}", default_name, i);
                    path = tmp_dir.join(name);
                    i += 1;
                } else {
                    return Err(e)
                },
        }
    }
}

/// Create parent dirs then atomically write/overwrite file with content,
/// using the tmp subdir of base for the tmp file.
pub async fn atomic_write<P>(
    base: &DataDir,
    path: P,
    content: &[u8]
) -> Result<()>
where
    P: AsRef<Path>,
{
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).await?;
    }
    let default_name = path.as_ref()
        .file_name()
        .expect("atomic_write to path with no file name")
        .to_string_lossy();
    let (mut tmp_file, tmp_path) = create_tmp_file(base, &default_name).await?;
    tmp_file.write_all(content).await?;
    drop(tmp_file);
    rename(tmp_path, path).await?;
    Ok(())
}
