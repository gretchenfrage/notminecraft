
use chunk_data::RawBlockId;
use std::ops::Index;


/// Association of `Option<T>` for each block.
///
/// (Not to be confused with each tile).
#[derive(Debug, Clone)]
pub struct PerBlock<T>(Vec<Option<T>>);

impl<T> PerBlock<T> {
    pub fn new() -> Self {
        PerBlock(Vec::new())
    }

    pub fn get<B: Into<RawBlockId>>(&self, bid: B) -> Option<&T> {
        self.0
            .get(bid.into().0 as usize)
            .and_then(|opt| opt.as_ref())
    }

    pub fn set<B: Into<RawBlockId>>(&mut self, bid: B, val: T) {
        let idx = bid.into().0 as usize;
        while self.0.len() < idx + 1 {
            self.0.push(None);
        }
        if self.0[idx].is_some() {
            warn!("overwrite of non-None PerBlock value");
        }
        self.0[idx] = Some(val)
    }
}
