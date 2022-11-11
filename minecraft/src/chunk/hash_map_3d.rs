
use std::{
	collections::hash_map::{
		HashMap,
		Entry,
	},
	hash::Hash,
	cell::Cell,
};
use vek::*;


// idx to diff:
//

/*
const NUM_NEIGHBORS: usize = 26;
const NIDX_TO_DIFF: [[i32; 3]; NUM_NEIGHBORS] = [
	[ 1, 0, 0], //  0
	[-1, 0, 0], //  1
	[ 0, 1, 0], //  2
	[ 0,-1, 0], //  3
	[ 0, 0, 1], //  4
	[ 0, 0,-1], //  5
	[ 1, 1, 0], //  6
	[-1, 1, 0], //  7
	[-1,-1, 0], //  8
	[ 1,-1, 0], //  9
	[ 1, 0, 1], // 10
	[-1, 0, 1], // 11
	[-1, 0,-1], // 12
	[ 1, 0,-1], // 13
	[ 0, 1, 1], // 14
	[ 0,-1, 1], // 15
	[ 0,-1,-1], // 16
	[ 0, 1,-1], // 17
	[ 1, 1, 1], // 18
	[-1, 1, 1], // 19
	[ 1,-1, 1], // 20
	[ 1, 1,-1], // 21
	[-1,-1, 1], // 22
	[-1, 1,-1], // 23
	[ 1,-1,-1], // 24
	[-1,-1,-1], // 25
];
const DIFF_TO_NIDX: [[[usize; 3]; 3]; 3] =
	[
		[
			[25,8,22],
			[12,1,11],
			[23,7,19],
		],
		[
			[16,3,15],
			[5,!0,4],
			[17,2,14],
		],
		[
			[24,9,20],
			[13,0,10],
			[21,6,18],
		],
	];
const NIDX_REV: [usize; NUM_NEIGHBORS] = [
//  0
//  1
//  2
//  3
//  4
//  5
//  6
//  7
//  8
//  9
// 10
// 11
// 12
// 13
// 14
// 15
// 16
// 17
// 18
// 19
// 20
// 21
// 22
// 23
// 24
// 25
];
const NULL_IDX: u32 = !0;*/

const NULL_IDX: u32 = !0;

const NUM_NEIGHBORS: usize = 26;
const NIDX_TO_DIFF: [[i32; 3]; NUM_NEIGHBORS] = [
	[ 1, 0, 0], //  0
	[-1, 0, 0], //  1
	[ 0, 1, 0], //  2
	[ 0,-1, 0], //  3
	[ 0, 0, 1], //  4
	[ 0, 0,-1], //  5

	[ 0, 1, 1], //  6
	[ 0,-1,-1], //  7
	[ 1, 0, 1], //  8
	[-1, 0,-1], //  9
	[ 1, 1, 0], // 10
	[-1,-1, 0], // 11
	[ 0,-1, 1], // 12
	[ 0, 1,-1], // 13
	[-1, 0, 1], // 14
	[ 1, 0,-1], // 15
	[-1, 1, 0], // 16
	[ 1,-1, 0], // 17

	[ 1, 1, 1], // 18
	[-1,-1,-1], // 19
	[-1, 1, 1], // 20
	[ 1,-1,-1], // 21
	[ 1,-1, 1], // 22
	[-1, 1,-1], // 23
	[ 1, 1,-1], // 24
	[-1,-1, 1], // 25
];
const DIFF_TO_NIDX: [[[usize; 3]; 3]; 3] =
	[
		[
			[19,11,25],
			[ 9, 1,14],
			[23,16,20],
		],
		[
			[ 7, 3,12],
			[ 5,!0, 4],
			[13, 2, 6],
		],
		[
			[21,17,22],
			[15, 0, 8],
			[24,10,18],
		],
	];


pub struct HashMap3d<T> {
	hmap: HashMap<Vec3<i32>, u32>,
	vals: Vec<Val<T>>,
}

struct Val<T> {
	val: T,
	neighbors: [u32; NUM_NEIGHBORS],
}


impl<T> HashMap3d<T>
where
	T: Default + Clone,
{
	pub fn new() -> Self {
		HashMap3d {
			hmap: HashMap::new(),
			vals: Vec::new(),
		}
	}

	pub fn insert(&mut self, key: Vec3<i32>, val: T) {
		let entry =
			match self.hmap.entry(key) {
				Entry::Vacant(vacant) => vacant,
				Entry::Occupied(_) => panic!("entry already occupied"),
			};

		assert!(self.vals.len() < NULL_IDX, "too many loaded chunks ");
		let idx = self.vals.len() as u16;

		let mut neighbors = [!0; NUM_NEIGHBORS];

		for nidx in 0..NUM_NEIGHBORS {
			let diff = Vec3::from(NIDX_TO_DIFF[nidx]);

			if let Some(idx2) = self.hmap.get(key + diff).copied() {
				let nidx2 = 

				neighbors[nidx] = idx2;
				self.vals[idx2 as usize].neighbors
			}
		}

		self.vals.push()
	}

	pub fn remove(&mut self, key: Vec3<i32>) {
		unimplemented!()
	}

	pub fn get(&self, key: Vec3<i32>) -> Option<&T> {
		unimplemented!()
	}

	pub fn get_mut(&mut self, key: Vec3<i32>) -> Option<&mut T> {
		unimplemented!()
	}

	pub fn getter(&self) -> Getter<T> {
		unimplemented!()
	}

	pub fn getter_mut(&mut self) -> GetterMut<T> {
		unimplemented!()
	}
	/*
	pub fn iter<'s>(
		&'s self,
	) -> impl Iterator<Item=(Vec3<i32>, T)> + 's
	{

	}

	pub fn iter_mut<'s>(
		&'s mut self,
	) -> impl Iterator<Item=(Vec3<i32>, T)> + 's
	{
		
	}
	*/
}

#[derive(Debug, Clone, Default)]
struct IdxCache(Cell<Option<(Vec3<i32>, u32)>>);

impl IdxCache {
	fn get<T>(
		&self,
		key: Vec3<i32>,
		hmap: &HashMap3d<T>,
	) -> Option<u32> {
		if let Some((cache_key, cache_idx)) = self.0.get() {
			// case 1: is cached
			// 
			// just return
			if cache_key == key {
				return Some(cache_idx);
			}

			// case 2: neighbor is cached
			//
			// traverse link, cache if Some, return
			
			let diff = key - cache_key;
			if diff.x >= -1
				&& diff.y >= -1
				&& diff.z >= -1
				&& diff.x <= 1
				&& diff.y <= 1
				&& diff.z <= 1
			{
				let neighbor_idx = DIFF_TO_NIDX
					[(diff.x + 1) as usize]
					[(diff.y + 1) as usize]
					[(diff.z + 1) as usize];

				let idx = hmap
					.vals[cache_idx as usize]
					.neighbors[neighbor_idx];

				return
					if idx == NULL_IDX { None }
					else {
						self.0.set(Some((key, idx)));
						Some(idx)
					};
			}

		}

		// case 3: not cached
		//
		// hashmap lookup, cache if Some, return
		let idx = hmap.hmap
			.get(&key)
			.copied();
		if let Some(idx) = idx {
			self.0.set(Some((key, idx)));
		} 
		idx
	}
}

pub struct Getter<'a, T> {
	hmap: &'a HashMap3d<T>,
	cache: IdxCache,
}

pub struct GetterMut<'a, T> {
	hmap: &'a mut HashMap3d<T>,
	cache: IdxCache,
}

impl<'a, T> Getter<'a, T> {
	fn get(&self, key: Vec3<i32>) -> Option<&'a T> {
		self.cache
			.get(key, self.hmap)
			.map(|i| &self.hmap.vals[i as usize].val)
	}
}

impl<'a, T> GetterMut<'a, T> {
	fn get_mut(&mut self, key: Vec3<i32>) -> Option<&mut T> {
		self.cache
			.get(key, self.hmap)
			.map(|i| &mut self.hmap.vals[i as usize].val)
	}
}
