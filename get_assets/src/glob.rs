
use crate::{
    name::AssetName,
    DataDir,
};
use tokio::fs::{
    self,
    read_dir,
};
use regex::{
    Regex,
    escape,
};


pub async fn match_assets(base: &DataDir, glob: &str) -> Option<Vec<Vec<u8>>> {
    let glob = AssetName::try_new(glob).unwrap();
    let (dir_path, file_glob) = glob.split_dir_file();

    let mut regex = String::new();
    let mut char_buf = [0; 4];
    for c in file_glob.chars() {
        match c {
            '*' => regex.push_str(".*"),
            _ => regex.push_str(&escape(c.encode_utf8(&mut char_buf))),
        }
    }
    let regex = Regex::new(&regex).unwrap();

    let dir_path = base.assets_subdir().join(dir_path);
    let mut read_dir = read_dir(&dir_path).await
        .map_err(|e| error!(
            %e,
            %glob,
            "error reading asset glob surrounding dir",
        ))
        .ok()?;

    let mut matches = Vec::new();

    loop {
        let dir_entry =
            match read_dir.next_entry().await {
                Ok(None) => break,
                Ok(Some(entry)) => entry,
                Err(e) => {
                    error!(
                        %e,
                        %glob,
                        "error reading asset glob dir entry",
                    );
                    continue;
                }
            };
        let file_name =
            match dir_entry.file_name().into_string() {
                Ok(name) => name,
                Err(_) => continue,
            };
        if !regex.is_match(&file_name) {
            continue;
        }
        trace!("found match for {}: {}", glob, file_name);
        let content =
            match fs::read(dir_entry.path()).await {
                Ok(content) => content,
                Err(e) => {
                    error!(
                        %e,
                        path=%dir_entry.path().display(),
                        "error reading asset glob-matched file"
                    );
                    continue;
                }
            };
        matches.push(content);
    }

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}
