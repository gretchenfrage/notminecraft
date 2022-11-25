
mod idx;


use self::idx::{
    Idx,
    PackedIdx,
    PackedIdxRepr,
};
use crate::MeshData;
use graphics::{
    GpuVecContext,
    frame_content::{
        Vertex,
        GpuVec,
        GpuVecElem,
    },
};
use std::collections::VecDeque;
use slab::Slab;


// ==== types to make the large number of index spaces less bad ====
// this has already caught several bugs

macro_rules! idx_newtype {
    ($n:ident)=>{
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        struct $n(Idx);

        impl $n {
            fn get(self) -> Idx {
                self.0
            }
        }
    };
}

idx_newtype!(OuterIdx);
idx_newtype!(VertexIdx);
idx_newtype!(TriangleIdx);
idx_newtype!(IndexIdx);

impl TriangleIdx {
    fn flatten(self) -> [IndexIdx; 3] {
        [0, 1, 2].map(|rem| IndexIdx(Idx::new(self.0.usize() * 3 + rem)))
    }
}

impl IndexIdx {
    fn unflatten(self) -> (TriangleIdx, usize) {
        (TriangleIdx(Idx::new(self.0.usize() / 3)), self.0.usize() % 3)
    }
}

macro_rules! option_idx_newtype_into_from_packed_idx {
    ($n:ident)=>{
        impl Into<PackedIdx> for Option<$n> {
            fn into(self) -> PackedIdx {
                match self {
                    Some($n(i)) => PackedIdx::new(false, i),
                    None => PackedIdx::new(true, Idx::new(0)),
                }
            }
        }

        impl From<PackedIdx> for Option<$n> {
            fn from(packed: PackedIdx) -> Self {
                match packed.hi_bit() {
                    false => Some($n(packed.idx())),
                    true => None,
                }
            }
        }
    };
}

option_idx_newtype_into_from_packed_idx!(VertexIdx);
option_idx_newtype_into_from_packed_idx!(IndexIdx);


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum VertexOrOuterIdx {
    Vertex(VertexIdx),
    Outer(OuterIdx),
}

impl Into<PackedIdx> for VertexOrOuterIdx {
    fn into(self) -> PackedIdx {
        match self {
            VertexOrOuterIdx::Vertex(VertexIdx(i)) => PackedIdx::new(false, i),
            VertexOrOuterIdx::Outer(OuterIdx(i)) => PackedIdx::new(true, i),
        }
    }
}

impl From<PackedIdx> for VertexOrOuterIdx {
    fn from(packed: PackedIdx) -> Self {
        match packed.hi_bit() {
            false => VertexOrOuterIdx::Vertex(VertexIdx(packed.idx())),
            true => VertexOrOuterIdx::Outer(OuterIdx(packed.idx())),
        }
    }
}


// ==== the differ ====

#[derive(Debug, Clone)]
pub struct MeshDiffer {
    outer: Slab<PackedIdxRepr<Option<VertexIdx>>>,
    
    vertices: Vec<VertexElem>,
    vertices_holes: VecDeque<VertexIdx>,
    vertices_writes: VecDeque<VertexIdx>,

    triangles: Vec<[IndexElem; 3]>,
    triangles_holes: VecDeque<TriangleIdx>,
    indices_writes: VecDeque<IndexIdx>,
}

#[derive(Debug, Copy, Clone)]
struct VertexElem {
    prev: PackedIdxRepr<VertexOrOuterIdx>,
    next: PackedIdxRepr<Option<VertexIdx>>,
    first_index: PackedIdxRepr<Option<IndexIdx>>,
    val: Vertex,

    #[cfg(debug_assertions)]
    garbage: bool,
}

#[derive(Debug, Copy, Clone)]
struct IndexElem {
    prev: PackedIdxRepr<Option<IndexIdx>>,
    next: PackedIdxRepr<Option<IndexIdx>>,
    val: VertexIdx,

    #[cfg(debug_assertions)]
    garbage: bool,
}


pub struct GpuVecDiff<I> {
    pub new_len: usize,
    pub writes: I,
}


impl MeshDiffer {
    pub fn new() -> Self {
        MeshDiffer {
            outer: Slab::new(),

            vertices: Vec::new(),
            vertices_holes: VecDeque::new(),
            vertices_writes: VecDeque::new(),

            triangles: Vec::new(),
            triangles_holes: VecDeque::new(),
            indices_writes: VecDeque::new(),
        }
    }

    pub fn add_submesh(&mut self, submesh: &MeshData) -> usize {
        submesh.validate_indices();

        let key = self.outer.insert(None.into());

        let mut prev = VertexOrOuterIdx::Outer(OuterIdx(Idx::new(key)));

        for &vertex in &submesh.vertices {
            #[cfg(not(debug_assertions))]
            let vertex_elem = VertexElem {
                prev: prev.into(),
                next: None.into(),
                first_index: None.into(),
                val: vertex,
            };
            #[cfg(debug_assertions)]
            let vertex_elem = VertexElem {
                prev: prev.into(),
                next: None.into(),
                first_index: None.into(),
                val: vertex,
                garbage: false,
            };

            let curr =
                if let Some(hole) = self.vertices_holes.pop_front() {
                    self.vertices[VertexIdx::get(hole).usize()] = vertex_elem;
                    hole
                } else {
                    let curr = VertexIdx(Idx::new(self.vertices.len()));
                    self.vertices.push(vertex_elem);
                    curr
                };

            self.vertices_writes.push_back(curr);

            match prev {
                VertexOrOuterIdx::Vertex(prev) => {
                    self
                        .vertices[VertexIdx::get(prev).usize()]
                        .next
                        = Some(curr).into();
                }
                VertexOrOuterIdx::Outer(prev) => {
                    self
                        .outer[OuterIdx::get(prev).usize()]
                        = Some(curr).into();
                }
            }

            prev = VertexOrOuterIdx::Vertex(curr);
        }

        for submesh_triangle in submesh.triangles() {
            let mesh_triangle = [0, 1, 2]
                .map(|rem| {
                    let submesh_index = submesh_triangle[rem];
                    let vertices_writes_queue_index =
                        self.vertices_writes.len()
                        - 1
                        - submesh_index;
                    self.vertices_writes[vertices_writes_queue_index]
                });
            let triangle_elem = mesh_triangle
                .map(|index| {
                    let next = self
                        .vertices[VertexIdx::get(index).usize()]
                        .first_index;

                    #[cfg(not(debug_assertions))]
                    let index_elem = IndexElem {
                        prev: None.into(),
                        next,
                        val: index,
                    };
                    #[cfg(debug_assertions)]
                    let index_elem = IndexElem {
                        prev: None.into(),
                        next,
                        val: index,
                        garbage: false,
                    };
                    index_elem
                });
            
            let curr =
                if let Some(hole) = self.triangles_holes.pop_front() {
                    self.triangles[TriangleIdx::get(hole).usize()] = triangle_elem;
                    hole
                } else {
                    let curr = TriangleIdx(Idx::new(self.triangles.len()));
                    self.triangles.push(triangle_elem);
                    curr
                };

            let index_idx_triangle = TriangleIdx::flatten(curr);
            self.indices_writes.extend(index_idx_triangle);

            for rem in 0..3 {
                let index = mesh_triangle[rem];
                let index_idx = index_idx_triangle[rem];
                let next = self
                    .vertices[VertexIdx::get(index).usize()]
                    .first_index
                    .unpack();
                if let Some(next) = next {
                    let (next_triangle, next_rem) = IndexIdx::unflatten(next);
                    self
                        .triangles[TriangleIdx::get(next_triangle).usize()][next_rem]
                        .prev
                        = Some(index_idx).into();
                }
                self
                    .vertices[VertexIdx::get(index).usize()]
                    .first_index
                    = Some(index_idx).into();
            }
        }

        key
    }

    pub fn remove_submesh(&mut self, key: usize) {
        let outer_idx = OuterIdx(Idx::new(key));

        let mut curr_vertex_idx = self
            .outer
            .remove(OuterIdx::get(outer_idx).usize())
            .unpack();

        while let Some(vertex_idx) = curr_vertex_idx {
            let mut curr_index_idx = self
                .vertices[VertexIdx::get(vertex_idx).usize()]
                .first_index
                .unpack();

            #[cfg(debug_assertions)]
            if let Some(index_idx) = curr_index_idx {
                let (triangle_idx, _) = IndexIdx::unflatten(index_idx);
                assert!(TriangleIdx::get(triangle_idx).usize() < self.triangles.len());
            }

            while let Some(index_idx) = curr_index_idx {
                let (triangle_idx, rem) = IndexIdx::unflatten(index_idx);

                if rem == 0 {
                    self.triangles_holes.push_back(triangle_idx);
                    
                    #[cfg(debug_assertions)]
                    for rem2 in 0..3 {
                        self
                            .triangles[TriangleIdx::get(triangle_idx).usize()][rem2]
                            .garbage
                            = true;
                    }
                }

                curr_index_idx = self
                    .triangles[TriangleIdx::get(triangle_idx).usize()][rem]
                    .next
                    .unpack();

                #[cfg(debug_assertions)]
                if let Some(index_idx) = curr_index_idx {
                    let (triangle_idx, _) = IndexIdx::unflatten(index_idx);
                    assert!(TriangleIdx::get(triangle_idx).usize() < self.triangles.len());
                }
            }

            self.vertices_holes.push_back(vertex_idx);

            #[cfg(debug_assertions)]
            {
                self.vertices[VertexIdx::get(vertex_idx).usize()].garbage = true;
            }

            curr_vertex_idx = self
                .vertices[VertexIdx::get(vertex_idx).usize()]
                .next
                .unpack();
        }
    }

    pub fn diff<'s>(&'s mut self) -> (
        GpuVecDiff<impl Iterator<Item=(usize, Vertex)> + 's>,
        GpuVecDiff<impl Iterator<Item=(usize, usize)> + 's>,
    ) {
        let mut virtual_vertices_len = self.vertices.len();

        'fill_hole: while let Some(hole) = self.vertices_holes.pop_front() {

            let mut hole = hole;
            while VertexIdx::get(hole).usize() >= virtual_vertices_len {
                hole = self
                    .vertices[VertexIdx::get(hole).usize()]
                    .next
                    .unpack()
                    .unwrap();
            }
            let hole = hole;

            if VertexIdx::get(hole).usize() + 1 == virtual_vertices_len {
                virtual_vertices_len -= 1;
            } else {
                
                self.vertices[VertexIdx::get(hole).usize()] = self.vertices[virtual_vertices_len - 1];
                self.vertices[virtual_vertices_len - 1].next = Some(hole).into();
                virtual_vertices_len -= 1;

                let moved_from = VertexIdx(Idx::new(virtual_vertices_len));
                
                #[cfg(debug_assertions)]
                let garbage = self.vertices[VertexIdx::get(hole).usize()].garbage;

                let prev = self.vertices[VertexIdx::get(hole).usize()].prev.unpack();
                let prev_next = match prev {
                    VertexOrOuterIdx::Vertex(prev) => {
                        if VertexIdx::get(prev).usize() < virtual_vertices_len {
                            &mut self.vertices[VertexIdx::get(prev).usize()].next
                        } else {
                            //debug!(?prev, %virtual_vertices_len);
                            #[cfg(debug_assertions)]
                            assert!(garbage);
                            continue 'fill_hole
                        }
                    } // hahahhahahahahhahahahhahah
                    VertexOrOuterIdx::Outer(prev) => {
                        match self.outer.get_mut(OuterIdx::get(prev).usize()) {
                            Some(prev_elem) => prev_elem,
                            None => {
                                #[cfg(debug_assertions)]
                                assert!(garbage);
                                continue 'fill_hole
                            }
                        }
                    }
                };
                let old_prev_next = Some(moved_from);
                let new_prev_next = Some(hole);
                if prev_next.unpack() == old_prev_next {
                    *prev_next = new_prev_next.into();
                } else {
                    //debug!(prev_next=?prev_next, ?old_prev_next, ?new_prev_next);
                    #[cfg(debug_assertions)]
                    assert!(garbage);
                    continue 'fill_hole;
                }

                let next = self.vertices[VertexIdx::get(hole).usize()].next.unpack();
                if let Some(next) = next {
                    let old_next_prev = VertexOrOuterIdx::Vertex(moved_from);
                    let new_next_prev = VertexOrOuterIdx::Vertex(hole);
                    let next_prev =
                        if VertexIdx::get(next).usize() < virtual_vertices_len {
                            &mut self.vertices[VertexIdx::get(next).usize()].prev
                        } else {
                            #[cfg(debug_assertions)]
                            assert!(garbage);
                            continue 'fill_hole
                        };
                    if next_prev.unpack() == old_next_prev {
                        *next_prev = new_next_prev.into();
                    } else {
                        #[cfg(debug_assertions)]
                        assert!(garbage);
                        continue 'fill_hole;
                    }
                }

                self.vertices_writes.push_back(hole);
                
                let mut curr_index_idx = self
                    .vertices[VertexIdx::get(hole).usize()]
                    .first_index
                    .unpack();

                while let Some(index_idx) = curr_index_idx {
                    let (triangle_idx, rem) = IndexIdx::unflatten(index_idx);

                    let index_val =
                        match self
                            .triangles
                            .get_mut(TriangleIdx::get(triangle_idx).usize())
                        {
                            Some(triangle_elem) => &mut triangle_elem[rem].val,
                            None => {
                                #[cfg(debug_assertions)]
                                assert!(garbage);
                                continue 'fill_hole
                            }
                        };
                    if *index_val == moved_from {
                        *index_val = hole;
                    } else {
                        #[cfg(debug_assertions)]
                        assert!(garbage);
                        continue 'fill_hole;
                    }

                    self.indices_writes.push_back(index_idx);

                    curr_index_idx = self
                        .triangles[TriangleIdx::get(triangle_idx).usize()][rem]
                        .next
                        .unpack();
                }
            }
        }

        while self.vertices.len() > virtual_vertices_len {
            self.vertices.pop().unwrap();
        }

        let mut virtual_triangles_len = self.triangles.len();

        'fill_hole: while let Some(hole) = self.triangles_holes.pop_front() {

            let mut hole = hole;
            while TriangleIdx::get(hole).usize() >= virtual_triangles_len {
                hole = TriangleIdx(IndexIdx::get(self
                    .triangles[TriangleIdx::get(hole).usize()][0]
                    .next
                    .unpack()
                    .unwrap()));
            }
            let hole = hole;

            if TriangleIdx::get(hole).usize() + 1 == virtual_triangles_len {
                virtual_triangles_len -= 1;
            } else {
                self.triangles[TriangleIdx::get(hole).usize()] = self.triangles[virtual_triangles_len - 1];
                self.triangles[virtual_triangles_len - 1][0].next = Some(IndexIdx(TriangleIdx::get(hole))).into();
                virtual_triangles_len -= 1;

                let moved_from_triangle =
                    TriangleIdx::flatten(TriangleIdx(Idx::new(virtual_triangles_len)));
                let moved_to_triangle =
                    TriangleIdx::flatten(hole);
                
                // TODO: could move this, and equivalents above, to end, to
                //       gain optmization for early continuing
                self.indices_writes.extend(moved_to_triangle);

                for rem in 0..3 {
                    let old = Some(moved_from_triangle[rem]);
                    let new = Some(moved_to_triangle[rem]);

                    #[cfg(debug_assertions)]
                    let garbage = self
                        .triangles[TriangleIdx::get(hole).usize()][rem]
                        .garbage;

                    let prev = self
                        .triangles[TriangleIdx::get(hole).usize()][rem]
                        .prev
                        .unpack();
                    if let Some(prev) = prev {
                        let (
                            prev_triangle_idx,
                            prev_rem,
                        ) = IndexIdx::unflatten(prev);
                        let prev_next =
                            if
                                TriangleIdx::get(prev_triangle_idx).usize()
                                < virtual_triangles_len
                            {
                                &mut self
                                    .triangles
                                    [TriangleIdx::get(prev_triangle_idx).usize()]
                                    [prev_rem]
                                    .next
                            } else {
                                #[cfg(debug_assertions)]
                                assert!(garbage);
                                continue 'fill_hole
                            };
                        if prev_next.unpack() == old {
                            *prev_next = new.into();
                        } else {
                            #[cfg(debug_assertions)]
                            assert!(garbage);
                            continue 'fill_hole;
                        }
                    } else {
                        let vertex_idx = self
                            .triangles[TriangleIdx::get(hole).usize()][rem]
                            .val;
                        let vertex_first_index =
                            if 
                                VertexIdx::get(vertex_idx).usize()
                                < self.vertices.len()
                            {
                                &mut self
                                    .vertices[VertexIdx::get(vertex_idx).usize()]
                                    .first_index
                            } else {
                                #[cfg(debug_assertions)]
                                assert!(garbage);
                                continue 'fill_hole
                            };
                        if vertex_first_index.unpack() == old {
                            *vertex_first_index = new.into();
                        } else {
                            #[cfg(debug_assertions)]
                            assert!(garbage);
                            continue 'fill_hole;
                        }
                    }

                    let next = self
                        .triangles[TriangleIdx::get(hole).usize()][rem]
                        .next
                        .unpack();
                    if let Some(next) = next {
                        let (
                            next_triangle_idx,
                            next_rem,
                        ) = IndexIdx::unflatten(next);
                        let next_prev =
                            if
                                TriangleIdx::get(next_triangle_idx).usize()
                                < virtual_triangles_len
                            {
                                &mut self
                                    .triangles
                                    [TriangleIdx::get(next_triangle_idx).usize()]
                                    [next_rem]
                                    .prev
                            } else {
                                #[cfg(debug_assertions)]
                                assert!(garbage);
                                continue 'fill_hole
                            };
                        if next_prev.unpack() == old {
                            *next_prev = new.into();
                        } else {
                            #[cfg(debug_assertions)]
                            assert!(garbage);
                            continue 'fill_hole;
                        }
                    }
                }
            }
        }

        while self.triangles.len() > virtual_triangles_len {
            self.triangles.pop().unwrap();
        }

        let vertices_diff_writes = self
            .vertices_writes
            .drain(..)
            .filter(|&vertex_idx|
                VertexIdx::get(vertex_idx).usize() < self.vertices.len())
            .map(|vertex_idx| (
                VertexIdx::get(vertex_idx).usize(),
                self.vertices[VertexIdx::get(vertex_idx).usize()].val,
            ));
        let vertices_diff = GpuVecDiff {
            new_len: self.vertices.len(),
            writes: vertices_diff_writes,
        };

        let indices_diff_writes = self
            .indices_writes
            .drain(..)
            .filter(|&index_idx|
                IndexIdx::get(index_idx).usize() < self.triangles.len() * 3)
            .map(|index_idx| (
                IndexIdx::get(index_idx).usize(),
                {
                    let (triangle_idx, rem) = IndexIdx::unflatten(index_idx);
                    let index = self
                        .triangles[TriangleIdx::get(triangle_idx).usize()][rem]
                        .val;
                    VertexIdx::get(index).usize()
                },
            ));
        let indices_diff = GpuVecDiff {
            new_len: self.triangles.len() * 3,
            writes: indices_diff_writes,
        };

        (vertices_diff, indices_diff)
    }
}


impl<T: GpuVecElem, I: Iterator<Item=(usize, T)>> GpuVecDiff<I> {
    pub fn patch<C>(self, gpu_vec: &mut GpuVec<T>, gpu_vec_context: &C)
    where
        C: GpuVecContext,
    {
        // TODO: this is wasteful

        gpu_vec_context.set_gpu_vec_len(gpu_vec, self.new_len);

        struct RangeStart {
            src_start: usize,
            dst_start: usize,
        }

        let mut range_starts: Vec<RangeStart> = Vec::new();
        let mut values: Vec<T> = Vec::new();

        let mut last_dst: Option<usize> = None;

        for (src, (dst, value)) in self.writes.enumerate() {
            if !last_dst.map(|last_dst| dst == last_dst + 1).unwrap_or(false) {
                range_starts.push(RangeStart {
                    src_start: src,
                    dst_start: dst,
                });
            }

            last_dst = Some(dst);

            values.push(value);
        }

        let mut patches: Vec<(usize, &[T])> = Vec::new();

        for (i, range_start) in range_starts.iter().enumerate() {
            let slice = match range_starts.get(i + 1) {
                Some(next_range_start) => {
                    &values[range_start.src_start..next_range_start.src_start]
                }
                None => {
                    &values[range_start.src_start..]
                }
            };
            patches.push((
                range_start.dst_start,
                slice,
            ));
        }

        gpu_vec_context.patch_gpu_vec(gpu_vec, &patches);
    }
}
