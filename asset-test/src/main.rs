
use std::path::PathBuf;
use anyhow::{
    Result,
    ensure,
    error,
};
use reqwest::get;
use url::Url;


const ASSET_INDEX_URL: &'static str =
    "https://launchermeta.mojang.com/v1/packages/3d8e55480977e32acd9844e545177e69a52f594b/pre-1.6.json";

const RESOURCE_URL_BASE: &'static str =
    "https://resources.download.minecraft.net";

const NAME_DELIMITER: char = '/';

mod model {
    use std::collections::BTreeMap;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct AssetIndex {
        pub objects: BTreeMap<String, AssetIndexObject>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct AssetIndexObject {
        pub hash: String,
    }
}

impl model::AssetIndex {
    fn iter_names_hashes<'a>(&'a self) -> impl Iterator<Item=(&'a str, &'a str)> + 'a
    {
        self.objects
            .iter()
            .map(|(name, object)| (name.as_str(), object.hash.as_str()))
    }
}

fn patdown_hash(hash: &str) -> Result<()> {
    ensure!(hash.len() == 40, "hash {:?} wrong len", hash);
    for c in hash.chars() {
        ensure!(
            matches!(c, '0'..='9' | 'a'..='f'),
            "illegal hash char {:?}",
            c,
        )
    }
    Ok(())
}

fn patdown_name_part(part: &str) -> Result<()> {
    for c in part.chars() {
        ensure!(
            matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.'),
            "illegal name part char {:?}",
            c ,
        );
    }
    Ok(())
}

fn resource_url(hash: &str) -> Result<Url> {
    patdown_hash(hash)?;
    let mut url = Url::parse(RESOURCE_URL_BASE).unwrap();
    let mut path = url.path_segments_mut().unwrap();
    path.push(&hash[..2]);
    path.push(hash);
    Ok(url)
}

#[tokio::main]
async fn main() -> Result<()> {
    for (name, hash) in
        get(ASSET_INDEX_URL).await?
            .json::<model::AssetIndex>().await?
            .iter_names_hashes()
    {
        let mut path = PathBuf::new("assets-download");

        println!("{:?}", (path, url));
    }

    Ok(())
}
