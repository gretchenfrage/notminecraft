
use chunk_data::RawBlockId;
use std::ops::Index;


/// Association of `Option<T>` for each block.
///
/// (Not to be confused with each tile).
#[derive(Debug, Clone)]
pub struct PerBlock<T> {
    vals: Vec<Option<T>>,
    default: Option<T>,
}

impl<T> PerBlock<T> {
    pub fn new(default: T) -> Self {
        PerBlock {
            vals: Vec::new(),
            default: Some(default),
        }
    }

    pub fn new_no_default() -> Self {
        PerBlock {
            vals: Vec::new(),
            default: None,
        }
    }

    pub fn get<B: Into<RawBlockId>>(&self, bid: B) -> &T {
        self.vals
            .get(bid.into().0 as usize)
            .and_then(|opt| opt.as_ref())
            .unwrap_or_else(|| self
                .default
                .as_ref()
                .expect("PerBlock::get with no val set and no default"))
    }

    pub fn set<B: Into<RawBlockId>>(&mut self, bid: B, val: T) {
        let idx = bid.into().0 as usize;
        while self.vals.len() < idx + 1 {
            self.vals.push(None);
        }
        if self.vals[idx].is_some() {
            warn!("overwrite of non-None PerBlock value");
        }
        self.vals[idx] = Some(val)
    }
}

impl<B: Into<RawBlockId>, T> Index<B> for PerBlock<T> {
    type Output = T;

    fn index(&self, bid: B) -> &T {
        self.get(bid)
    }
}
