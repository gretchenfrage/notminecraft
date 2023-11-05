
use crate::{
    name::AssetName,
    download::index::hash::HashStr,
};
use anyhow::Result;


/// Json object structure.
mod obj {
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

/// Convert + validate downloaded asset index JSON to (name, hash) entries.
pub fn deser_index(
    json: &[u8],
) -> Result<Vec<(AssetName<String>, HashStr<String>)>> {
    let obj = serde_json::from_slice::<obj::AssetIndex>(json)?;
    let mut entries = Vec::new();
    for (name, obj::AssetIndexObject { hash }) in obj.objects {
        let name = AssetName::try_new(name)?;
        let hash = HashStr::try_new(hash)?;
        entries.push((name, hash))
    }
    Ok(entries)
}
