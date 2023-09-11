
#[derive(Debug, Clone)]
pub struct SparseFlags(Vec<u8>);

impl SparseFlags {
    pub fn new() -> Self {
        SparseFlags(Vec::new())
    }

    pub fn set(&mut self, i: usize) {
        let quo = i / 8;
        let rem = i % 8;

        while quo >= self.0.len() {
            self.0.push(0);
        }
        self.0[quo] |= 1 << rem;
    }

    pub fn unset(&mut self, i: usize) {
        let quo = i / 8;
        let rem = i % 8;

        if let Some(n) = self.0.get_mut(quo) {
            *n &= !(1 << rem);
        }
    }

    pub fn get(&self, i: usize) -> bool {
        let quo = i / 8;
        let rem = i % 8;

        self.0.get(quo)
            .map(|&n| n & (1 << rem) != 0)
            .unwrap_or(false)
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item=usize> + 'a {
        self.0.iter()
            .enumerate()
            .flat_map(|(quo, &n)| (0..8)
                .filter(move |rem| n & (1 << rem) != 0)
                .map(move |rem| quo * 8 + rem))
    }
}
