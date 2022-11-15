
use std::collections::VecDeque;
use slab::Slab;


pub trait MeshPatcher<T>: Sized {
	type Patching: MeshPatcherPatching<Self, T>;
	type Finalized;

	fn start_patch(self, dst_start_idx: u64) -> Self::Patching;

	fn finalize(self, final_len: usize) -> Self::Finalized;
}

pub trait MeshPatcherPatching<P, T>: Sized {
	fn write(&mut self, elem: &T);

	fn stop_patch(self) -> P;
}


const PACKED_IDX_HI_BIT: usize = 1_usize.rotate_right(1);

/// Bit-packed (1-bit bool, 63-bit u-int) tuple (assuming 64-bit CPU).
#[derive(Debug, Copy, Clone)]
struct PackedIdx(usize);

impl PackedIdx {
	fn new(hi_bit: bool, idx: usize) -> Self {
		assert!((idx & PACKED_IDX_HI_BIT) == 0, "idx too large");
		PackedIdx((hi_bit as usize).rotate_right(1) | idx)
	}

	fn hi_bit(self) -> bool {
		(self.0 & PACKED_IDX_HI_BIT) != 0
	}

	fn idx(self) -> usize {
		self.0 & !PACKED_IDX_HI_BIT
	}
}


#[derive(Debug, Clone)]
pub struct MeshDiffer<T> {
	inner: Vec<InnerElem<T>>,
	// if hi-bit == 0, is inner idx of first elem
	// if hi-bit == 1, submesh is empty
	outer: Slab<PackedIdx>,

	holes: VecDeque<usize>,
	writes: VecDeque<usize>,
}

#[derive(Debug, Clone)]
struct InnerElem<T> {
	// if hi-bit == 0, is inner idx of prev elem
	// if hi-bit == 1, is outer idx of submesh
	prev: PackedIdx,
	// if hi-bit == 0, is inner idx of next elem
	// if hi-bit == 1, this is last elem of submesh
	next: PackedIdx,
	elem: T,
}

impl<T> MeshDiffer<T> {
	pub fn new() -> Self {
		MeshDiffer {
			inner: Vec::new(),
			outer: Slab::new(),
			holes: VecDeque::new(),
			writes: VecDeque::new(),
		}
	}

	pub fn add_submesh<I>(&mut self, elems: I) -> usize
	where
		I: IntoIterator<Item=T>,
	{
		let key = self.outer.insert(PackedIdx::new(true, 0));

		let mut prev_hi_bit = true;
		let mut prev_idx = key;

		for elem in elems {
			// create new element
			let curr_elem = InnerElem {
				prev: PackedIdx::new(prev_hi_bit, prev_idx),
				next: PackedIdx::new(true, 0),
				elem,
			};

			// put in inner array, filling hole if possible
			let curr_idx = 
				if let Some(hole) = self.holes.pop_front() {
					// fill hole
					self.inner[hole] = curr_elem;
					hole
				} else {
					// push to end
					let idx = self.inner.len();
					self.inner.push(curr_elem);
					idx
				};

			// make note of write
			self.writes.push_back(curr_idx);

			// update prev element (whether inner or outer)'s "next" value
			if prev_hi_bit {
				self.outer[prev_idx] = PackedIdx::new(false, curr_idx);
			} else {
				self.inner[prev_idx].next = PackedIdx::new(false, curr_idx);
			}

			// prepare for next loop
			prev_hi_bit = false;
			prev_idx = curr_idx;
		}

		key
	}

	pub fn remove_submesh(&mut self, key: usize)
	{
		// remove from outer array
		let mut idx = self.outer.remove(key);
		
		// traverse inner array links, marking all submesh elems as holes
		while !idx.hi_bit() {
			self.holes.push_back(idx.idx());
			idx = self.inner[idx.idx()].next;
		}
	}

	pub fn diff<P>(&mut self, mut patcher: P) -> P::Finalized
	where
		P: MeshPatcher<T>,
	{
		// patch outstanding holes via swap-removal with idx updating
		while let Some(hole) = self.holes.pop_front()
		{
			if hole + 1 == self.inner.len() {
				// trivial case where we just pop
				self.inner.pop();
			} else {
				// case where we actually have to swap-remove
				self.inner.swap_remove(hole);

				// replace moved elem's prev elem's "next" index
				let prev = self.inner[hole].prev;
				if !prev.hi_bit() {
					// prev elem is inner
					self.inner[prev.idx()].next = PackedIdx::new(false, hole);
				} else {
					// prev elem is outer
					self.outer[prev.idx()] = PackedIdx::new(false, hole);
				}

				// replace moved elem's next elem's "prev" index
				let next = self.inner[hole].next;
				if !next.hi_bit() {
					// next elem exists
					self.inner[next.idx()].prev = PackedIdx::new(false, hole);
				}

				// make note of write
				self.writes.push_back(hole);
			}
		}

		// write out patches, recognizing contiguous ranges
		while let Some(i) = self.writes.pop_front()
		{
			// begin a patch and write the first element
			let mut patch = patcher.start_patch(u64::try_from(i).unwrap());
			if let Some(elem) = self.inner.get(i) {
				patch.write(&elem.elem);
			}

			// write further elements in this same patch for as long as it's
			// contiguous
			for j in ((i + 1)..)
				.take_while(|j|
					if self.writes.front() == Some(&j) {
						self.writes.pop_front().unwrap();
						true
					} else { false }
				)
			{
				if let Some(elem) = self.inner.get(j) {
					patch.write(&elem.elem);
				}
			}

			// finalize that patch
			patcher = patch.stop_patch();
		}

		// finalize
		patcher.finalize(self.inner.len())
	}
}

impl<T> Default for MeshDiffer<T> {
	fn default() -> Self {
		MeshDiffer::new()
	}
}
