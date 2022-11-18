
use std::{
	collections::VecDeque,
	mem::replace,
};
use slab::Slab;

/*
pub trait MeshPatcher<T>: Sized {
	type Patching: MeshPatcherPatching<Self, T>;
	type TriPhase: MeshPatcherTriPhase;

	fn start_patch(self, dst_start_idx: u64) -> Self::Patching;

	fn enter_tri_phase(self) -> Self::TriPhase;
}

pub trait MeshPatcherPatching<P, T>: Sized {
	fn write(&mut self, elem: &T);

	fn stop_patch(self) -> P;
}

pub trait MeshPatcherTriPhase: Sized {
	type Patching: MeshPatcherTriPhasePatching<Self>;
	type Finalized;

	fn start_patch(self, dst_start_idx: u64) -> Self::Patching;

	fn finalize(self, final_len: usize) -> Self::Finalized;
}

pub trait MeshPatcherTriPhasePatching<P>: Sized {
	fn write(&mut self, tri: [u64; 3]);

	fn stop_patch(self) -> P;
}
*/

pub trait PatcherTriPhase<T>: Sized {
	type TriPatchingPhase: PatcherTriPatchingPhase<Self>;
	type VertPhase: PatcherVertPhase<T>;

	fn start_patch(self, dst_start_idx: u64) -> Self::TriPatchingPhase;

	fn finalize_tri_phase(self, final_num_tris: u64) -> Self::VertPhase;
}

pub trait PatcherTriPatchingPhase<P>: Sized {
	fn write(&mut self, tri: [u64; 3]);

	fn stop_patch(self) -> P;
}

pub trait PatcherVertPhase<T>: Sized {
	type VertPatchingPhase: PatcherVertPatchingPhase<Self, T>;
	type Finalized;

	fn start_patch(self, dst_start_idx: u64) -> Self::VertPatchingPhase;

	fn finalize_vert_phase(self, final_num_verts: u64) -> Self::Finalized;
}

pub trait PatcherVertPatchingPhase<P, T>: Sized {
	fn write(&mut self, vert: T);

	fn stop_patch(self) -> P;
}


enum TriCombiner<T, P: PatcherTriPhase<T>> {
	NotPatching(P),
	Patching {
		patcher: <P as PatcherTriPhase<T>>::TriPatchingPhase,
		next_dst_idx: u64,
	},
}

impl<T, P: PatcherTriPhase<T>> TriCombiner<T> {
	fn new(patcher: P) -> Self {
		TriCombiner::NotPatching(patcher)
	}

	fn write(self, dst_idx: u64, tri: [u64; 3]) -> Self {
		match self {
			TriCombiner::NotPatching(patcher) => {
				let mut patcher = patcher.start_patch(dst_idx);
				patcher.write(tri);

				TriCombiner::Patching {
					patcher,
					next_dst_idx: dst_idx + 1,
				}
			}
			TriCombiner::Patching {
				mut patcher,
				mut next_dst_idx,
			} => {
				if dst_idx == next_dst_idx {
					patcher.write(tri);
					next_dst_idx += 1;

					TriCombiner::Patching { patcher, next_dst_idx }
				} else {
					let patcher = patcher.stop_patch()
					TriCombiner::NotPatching(patcher).write(dst_idx, tri)
				}
			}
		}
	}

	fn finalize_tri_phase(self) -> <P as PatcherTriPhase<T>>::VertPhase {
		let patcher = match self {
			TriCombiner::NotPatching(patcher) => patcher,
			TriCombiner::Patching { patcher, .. } => patcher.stop_patch(),
		};
		patcher.finalize_tri_phase()
	}
}


const PACKED_IDX_HI_BIT: usize = 1_usize.rotate_right(1);

/// Bit-packed (1-bit bool, 63-bit u-int) tuple (assuming 64-bit CPU).
#[derive(Debug, Copy, Clone, Default)]
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
	// if hi-bit == 0, is first vbuf idx
	// if hi-bit == 1, no vbuf contents
	outer: Slab<PackedIdx>,
	vbuf: Vec<VBufElem<T>>,
	ibuf: Vec<IBufTri>,

	vbuf_holes: VecDeque<usize>,
	vbuf_writes: VecDeque<usize>,

	// these queues use ibuf tri indices, not elem indices
	ibuf_holes: VecDeque<usize>,
	//ibuf_writes: VecDeque<usize>,
}

#[derive(Debug, Clone)]
struct VBufElem<T> {
	// if hi-bit == 0, is prev vbuf idx
	// if hi-bit == 1, is outer idx
	prev: PackedIdx,
	// if hi-bit == 0, is next vbuf idx
	// if hi-bit == 1, no further vbuf contents
	next: PackedIdx,
	// if hi-bit == 0, is first ibuf elem idx
	// if hi-bit == 1, no ibuf contents
	ibuf: PackedIdx,
	val: T,
}

type IBufTri = [IBufElem; 3];

#[derive(Debug, Copy, Clone)]
struct IBufElem {
	// if hi-bit == 0, is prev ibuf elem idx
	// if hi-bit == 1, no previous ibuf contents
	prev: PackedIdx,
	// if hi-bit == 0, is next ibuf elem idx
	// if hi-bit == 1, no further ibuf contents
	next: PackedIdx,
	val: usize,
}

impl<T> MeshDiffer<T> {
	pub fn new() -> Self {
		MeshDiffer {
			outer: Slab::new(),
			vbuf: Vec::new(),
			ibuf: Vec::new(),

			vbuf_holes: VecDeque::new(),
			vbuf_writes: VecDeque::new(),

			ibuf_holes: VecDeque::new(),
		}
	}

	pub fn add_submesh<I1, I2>(
		&mut self,
		verts: I1,
		triangles: I2,
	) -> usize
	where
		I1: IntoIterator<Item=T>,
		I2: IntoIterator<Item=[usize; 3]>,
	{
		// insert vertices
		let outer_idx = self.outer.insert(PackedIdx::new(true, 0));

		let mut prev_hi_bit = true;
		let mut prev_idx = outer_idx;

		// for triangle validation
		let mut submesh_num_verts = 0;

		for vert in verts {
			submesh_num_verts += 1;

			// create vbuf new element
			let vbuf_elem = VBufElem {
				prev: PackedIdx::new(prev_hi_bit, prev_idx),
				next: PackedIdx::new(true, 0),
				ibuf: PackedIdx::new(true, 0),
				val: vert,
			};

			// put in vbuf, filling hole if possible
			let curr_idx =
				if let Some(hole) = self.vbuf_holes.pop_front() {
					// fill hole
					self.vbuf[hole] = vbuf_elem;
					hole
				} else {
					// push to end
					let idx = self.vbuf.len();
					self.vbuf.push(vbuf_elem);
					idx
				};

			// make note of write
			self.vbuf_writes.push_back(curr_idx);

			// update prev's "next" idx
			if !prev_hi_bit {
				// prev is in vbuf
				self.vbuf[prev_idx].next = PackedIdx::new(false, curr_idx);
			} else {
				// prev is in outer
				self.outer[prev_idx] = PackedIdx::new(false, curr_idx);
			}
		}

		// insert triangles
		for submesh_rel_tri in triangles {
			// validate
			for submesh_rel_idx in submesh_rel_tri {
				assert!(
					submesh_rel_idx < submesh_num_verts,
					"index out of range",
				);
			}

			// determine what ibuf tri idx will be used
			let (fill_hole, ibuf_tri_idx) =
				if let Some(hole) = self.ibuf_holes.pop_front() {
					(true, hole)
				} else {
					(false, self.ibuf.len())
				};

			// construct the ibuf tri, and link
			let ibuf_tri = [0, 1, 2]
				.map(|tri_rel_idx| {
					// map abstracted index to vbuf idx without requiring
					// additional data structures by exploiting the fact that
					// we've already logged this mapping to the end of the
					// vbuf_writes queue
					let submesh_rel_idx = submesh_rel_tri[tri_rel_idx];
					let queue_idx = submesh_num_verts - 1 - submesh_rel_idx;
					let vbuf_idx = self.vbuf_writes[queue_idx];

					// link and construct ibuf elem
					let next = self.vbuf[vbuf_idx].ibuf;
					let curr = PackedIdx::new(
						false,
						ibuf_tri_idx * 3 + tri_rel_idx,
					);
					self.vbuf[vbuf_idx].ibuf = curr;
					if !next.hi_bit() {
						let next_idx = next.idx();
						self.ibuf[next_idx / 3][next_idx % 3].prev = curr;
					}

					IBufElem {
						prev: PackedIdx::new(true, 0),
						next,
						val: vbuf_idx,
					}
				});

			// put the ibuf tri in there
			if fill_hole {
				self.ibuf[ibuf_tri_idx] = ibuf_tri;
			} else {
				self.ibuf.push(ibuf_tri);
			}
		}

		// done
		outer_idx
	}

	pub fn remove_submesh(&mut self, key: usize)
	{
		// remove from the outer slab
		let mut vbuf_idx = self.outer.remove(key);

		// traverse through its vbuf links, marking each vbuf elem as a hole
		while !vbuf_idx.hi_bit() {
			self.vbuf_holes.push_back(vbuf_idx.idx());

			// within each vbuf iteration, traverse through the ibuf links,
			// marking each ibuf tri as a hole
			let mut ibuf_idx = self.vbuf[vbuf_idx.idx()].ibuf;

			while !ibuf_idx.hi_bit() {
				let ibuf_idx_quo = ibuf_idx.idx() / 3;
				let ibuf_idx_rem = ibuf_idx.idx() % 3;

				// only mark the ibuf tri as a hole when passing through the
				// first ibuf element within that tri, or else we'll mark
				// each tri as a hole 3 redundant times, which would cause
				// corruptions
				if ibuf_idx_rem == 0 {
					self.ibuf_holes.push_back(ibuf_idx_quo);
				}

				ibuf_idx = self.ibuf[ibuf_idx_quo][ibuf_idx_rem].next;
			}

			vbuf_idx = self.vbuf[vbuf_idx.idx()].next;
		}
	}

	pub fn diff(
		&mut self,
		patcher: P,
	) -> <<P as PatcherTriPhase<T>>::VertPhase as PatcherVertPhase<T>>::Finalized
	where
		P: PatcherTriPhase<T>,
	{
		let mut patcher = TriCombiner::new(patcher);

		let final_num_tris = self.ibuf.len() - self.ibuf_holes.len();

		while let Some(hole) = self.vbuf_holes.pop_front()
		{
			if hole + 1 = self.vbuf.len() {
				self.vbuf.pop().unwrap();
			} else {
				self.vbuf.swap_remove(hole);

				let prev = self.vbuf[hole].prev;
				if !prev.hi_bit() {
					self.vbuf[prev.idx()].next = PackedIdx::new(false, hole);
				} else {
					self.outer[prev.idx()] = PackedIdx::new(false, hole);
				}

				let next = self.vbuf[hole].next;
				if !next.hi_bit() {
					self.vbuf[next.idx()].prev = PackedIdx::new(false, hole)
				}

				let mut ibuf_idx = self.vbuf[hole].ibuf;
				while !ibuf_idx.hi_bit() {
					let ibuf_idx_quo = ibuf_idx.idx() / 3;
					let ibuf_idx_rem = ibuf_idx.idx() % 3;
					self.ibuf[ibuf_idx_quo][ibuf_idx_rem].val = hole;

					if ibuf_idx 
					todo!(); // TODO emit this ibuf write here
					
					ibuf_idx = self.ibuf[ibuf_idx_quo][ibuf_idx_rem].next;
				}

				self.writes.push_back(hole);
			}
		}

		while let Some(hole) = self.ibuf_holes.pop_front() {
			if hole + 1 == self.ibuf.len() {
				self.ibuf.pop().unwrap();
			} else {
				self.ibuf.swap_remove(hole);

				for tri_rel_idx in 0..3 {
					let curr = PackedIdx::new(hole * 3 + tri_rel_idx);

					let prev = self.ibuf[hole][tri_rel_idx].prev;
					if !prev.hi_bit() {
						let prev_quo = prev.idx() / 3;
						let prev_rem = prev.idx() % 3;
						self.ibuf[prev_quo][prev_rem].next = curr;
					}

					let next = self.ibuf[hole][tri_rel_idx].next;
					if !next.hi_bit() {
						let next_quo = next.idx() / 3;
						let next_rem = next.idx() % 3;
						self.ibuf[next_quo][next_rem].prev = curr;
					}
				}

				// TODO emit this ibuf write here
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
/*
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
*/
