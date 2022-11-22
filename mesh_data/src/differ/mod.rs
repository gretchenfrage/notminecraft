
mod packed_idx;


use self::packed_idx::{
    PackedIdx,
    PackedIdxRepr,
};
use crate::MeshData;
use graphics::{
    Renderer,
    frame_content::{
        Vertex,
        GpuVec,
        GpuVecElem,
    },
};
use std::{
    collections::VecDeque,
    fmt::{self, Formatter, Debug},
};
use slab::Slab;


// ==== types to make the large number of index spaces less bad ====
// this has already caught several bugs

macro_rules! usize_newtype {
    ($n:ident)=>{
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        struct $n(usize);

        impl $n {
            fn get(self) -> usize {
                self.0
            }
        }
    };
}

usize_newtype!(OuterIdx);
usize_newtype!(VertexIdx);
usize_newtype!(TriangleIdx);
usize_newtype!(IndexIdx);

impl TriangleIdx {
    fn flatten(self) -> [IndexIdx; 3] {
        [0, 1, 2].map(|rem| IndexIdx(self.0 * 3 + rem))
    }
}

impl IndexIdx {
    fn unflatten(self) -> (TriangleIdx, usize) {
        (TriangleIdx(self.0 / 3), self.0 % 3)
    }
}

macro_rules! option_usize_newtype_into_from_packed_idx {
    ($n:ident)=>{
        impl Into<PackedIdx> for Option<$n> {
            fn into(self) -> PackedIdx {
                match self {
                    Some($n(i)) => PackedIdx::new(false, i),
                    None => PackedIdx::new(true, 0),
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

option_usize_newtype_into_from_packed_idx!(VertexIdx);
option_usize_newtype_into_from_packed_idx!(IndexIdx);


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
}

#[derive(Debug, Copy, Clone)]
struct IndexElem {
    prev: PackedIdxRepr<Option<IndexIdx>>,
    next: PackedIdxRepr<Option<IndexIdx>>,
    val: VertexIdx,
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

        let mut prev = VertexOrOuterIdx::Outer(OuterIdx(key));

        for &vertex in &submesh.vertices {
            let vertex_elem = VertexElem {
                prev: prev.into(),
                next: None.into(),
                first_index: None.into(),
                val: vertex,
            };

            let curr =
                if let Some(hole) = self.vertices_holes.pop_front() {
                    todo!()/*
                    self.vertices[VertexIdx::get(hole)] = vertex_elem;
                    hole*/
                } else {
                    let curr = VertexIdx(self.vertices.len());
                    self.vertices.push(vertex_elem);
                    curr
                };

            self.vertices_writes.push_back(curr);

            match prev {
                VertexOrOuterIdx::Vertex(prev) => {
                    self
                        .vertices[VertexIdx::get(prev)]
                        .next
                        = Some(curr).into();
                }
                VertexOrOuterIdx::Outer(prev) => {
                    self
                        .outer[OuterIdx::get(prev)]
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
                        .vertices[VertexIdx::get(index)]
                        .first_index;

                    IndexElem {
                        prev: None.into(),
                        next,
                        val: index,
                    }
                });
            
            let curr =
                if let Some(hole) = self.triangles_holes.pop_front() {
                    todo!()/*
                    self.triangles[TriangleIdx::get(hole)] = triangle_elem;
                    hole*/
                } else {
                    let curr = TriangleIdx(self.triangles.len());
                    self.triangles.push(triangle_elem);
                    curr
                };

            let index_idx_triangle = TriangleIdx::flatten(curr);
            self.indices_writes.extend(index_idx_triangle);

            for rem in 0..3 {
                let index = mesh_triangle[rem];
                let index_idx = index_idx_triangle[rem];
                let next = self
                    .vertices[VertexIdx::get(index)]
                    .first_index
                    .unpack();
                if let Some(next) = next {
                    let (next_triangle, next_rem) = IndexIdx::unflatten(next);
                    self
                        .triangles[TriangleIdx::get(next_triangle)][next_rem]
                        .prev
                        = Some(index_idx).into();
                }
                self
                    .vertices[VertexIdx::get(index)]
                    .first_index
                    = Some(index_idx).into();
            }
        }

        key
    }

    pub fn remove_submesh(&mut self, key: usize) {
        todo!()/*
        let outer_idx = OuterIdx(key);

        let mut curr_vertex_idx = self
            .outer
            .remove(OuterIdx::get(outer_idx))
            .unpack();

        while let Some(vertex_idx) = curr_vertex_idx {
            let mut curr_index_idx = self
                .vertices[VertexIdx::get(vertex_idx)]
                .first_index
                .unpack();

            while let Some(index_idx) = curr_index_idx {
                let (triangle_idx, rem) = IndexIdx::unflatten(index_idx);

                if rem == 0 {
                    self.triangles_holes.push_back(triangle_idx);
                }

                curr_index_idx = self
                    .triangles[TriangleIdx::get(triangle_idx)][rem]
                    .next
                    .unpack();
            }

            self.vertices_holes.push_back(vertex_idx);

            curr_vertex_idx = self
                .vertices[VertexIdx::get(vertex_idx)]
                .next
                .unpack();
        }*/
    }

    pub fn diff<'s>(&'s mut self) -> (
        GpuVecDiff<impl Iterator<Item=(usize, Vertex)> + 's>,
        GpuVecDiff<impl Iterator<Item=(usize, usize)> + 's>,
    ) {
        while let Some(hole) = self.vertices_holes.pop_front() {
            todo!()/*
            if VertexIdx::get(hole) + 1 == self.vertices.len() {
                self.vertices.pop().unwrap();
            } else {
                self.vertices.swap_remove(VertexIdx::get(hole));

                let prev = self.vertices[VertexIdx::get(hole)].prev.unpack();
                let prev_next = Some(hole).into();
                match prev {
                    VertexOrOuterIdx::Vertex(prev) => {
                        self.vertices[VertexIdx::get(prev)].next = prev_next;
                    }
                    VertexOrOuterIdx::Outer(prev) => {
                        self.outer[OuterIdx::get(prev)] = prev_next;
                    }
                }

                let next = self.vertices[VertexIdx::get(hole)].next.unpack();
                if let Some(next) = next {
                    let next_prev = VertexOrOuterIdx::Vertex(hole).into();
                    self.vertices[VertexIdx::get(next)].prev = next_prev;
                }

                self.vertices_writes.push_back(hole);
                
                let mut curr_index_idx = self
                    .vertices[VertexIdx::get(hole)]
                    .first_index
                    .unpack();

                while let Some(index_idx) = curr_index_idx {
                    let (triangle_idx, rem) = IndexIdx::unflatten(index_idx);

                    self
                        .triangles[TriangleIdx::get(triangle_idx)][rem]
                        .val
                        = hole;
                    self.indices_writes.push_back(index_idx);

                    curr_index_idx = self
                        .triangles[TriangleIdx::get(triangle_idx)][rem]
                        .next
                        .unpack();
                }
            }*/
        }

        while let Some(hole) = self.triangles_holes.pop_front() {
            todo!()/*
            if TriangleIdx::get(hole) + 1 == self.triangles.len() {
                self.triangles.pop().unwrap();
            } else {
                self.triangles.swap_remove(TriangleIdx::get(hole));

                let index_idx_triangle = TriangleIdx::flatten(hole);
                self.indices_writes.extend(index_idx_triangle);

                for rem in 0..3 {
                    let curr = Some(index_idx_triangle[rem]).into();

                    let prev = self
                        .triangles[TriangleIdx::get(hole)][rem]
                        .prev
                        .unpack();
                    if let Some(prev) = prev {
                        let (
                            prev_triangle_idx,
                            prev_rem,
                        ) = IndexIdx::unflatten(prev);
                        self
                            .triangles
                            [TriangleIdx::get(prev_triangle_idx)]
                            [prev_rem]
                            .next
                            = curr;
                    }

                    let next = self
                        .triangles[TriangleIdx::get(hole)][rem]
                        .next
                        .unpack();
                    if let Some(next) = next {
                        let (
                            next_triangle_idx,
                            next_rem,
                        ) = IndexIdx::unflatten(next);
                        self
                            .triangles
                            [TriangleIdx::get(next_triangle_idx)]
                            [next_rem]
                            .prev
                            = curr;
                    }
                }
            }*/
        }

        let vertices_diff_writes = self
            .vertices_writes
            .drain(..)
            .filter(|&vertex_idx|
                VertexIdx::get(vertex_idx) < self.vertices.len())
            .map(|vertex_idx| (
                VertexIdx::get(vertex_idx),
                self.vertices[VertexIdx::get(vertex_idx)].val,
            ));
        let vertices_diff = GpuVecDiff {
            new_len: self.vertices.len(),
            writes: vertices_diff_writes,
        };

        let indices_diff_writes = self
            .indices_writes
            .drain(..)
            .filter(|&index_idx|
                IndexIdx::get(index_idx) < self.triangles.len() * 3)
            .map(|index_idx| (
                IndexIdx::get(index_idx),
                {
                    let (triangle_idx, rem) = IndexIdx::unflatten(index_idx);
                    let index = self
                        .triangles[TriangleIdx::get(triangle_idx)][rem]
                        .val;
                    VertexIdx::get(index)
                },
            ));
        let indices_diff = GpuVecDiff {
            new_len: self.triangles.len() * 3,
            writes: indices_diff_writes,
        };

        (vertices_diff, indices_diff)
    }

    pub fn alt_debug_1<'s>(&'s self) -> impl Debug + 's {
        AltDebug1(self)
    }

    pub fn alt_debug_2<'s>(&'s self) -> impl Debug + 's {
        AltDebug2(self)
    }
}


struct AltDebug1<'a>(&'a MeshDiffer);

impl<'a> Debug for AltDebug1<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_map();
        for (key, first_vertex_idx) in self.0.outer.iter() {
            let first_vertex_idx = first_vertex_idx.unpack();

            if let Some(first_vertex_idx) = first_vertex_idx {
                assert_eq!(
                    self.0.vertices[VertexIdx::get(first_vertex_idx)].prev.unpack(),
                    VertexOrOuterIdx::Outer(OuterIdx(key)),
                );
            }

            f.entry(
                &key,
                &AltDebug1Submesh {
                    differ: self.0,
                    first_vertex_idx,
                },
            );
        }
        f.finish()
    }
}

struct AltDebug1Submesh<'a> {
    differ: &'a MeshDiffer,
    first_vertex_idx: Option<VertexIdx>,
}

impl<'a> Debug for AltDebug1Submesh<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_list();

        let mut curr_vertex_idx = self.first_vertex_idx;

        while let Some(vertex_idx) = curr_vertex_idx {
            let mut curr_index_idx = self.differ.vertices[VertexIdx::get(vertex_idx)].first_index.unpack();

            if let Some(first_index_idx) = curr_index_idx {
                let (first_triangle_idx, first_rem) = IndexIdx::unflatten(first_index_idx);

                assert_eq!(
                    self.differ.triangles[TriangleIdx::get(first_triangle_idx)][first_rem].prev.unpack(),
                    None,
                );
            }

            while let Some(index_idx) = curr_index_idx {
                let (curr_triangle_idx, curr_rem) = IndexIdx::unflatten(index_idx);

                if curr_rem == 0 {
                    f.entry(&AltDebug1Triangle {
                        differ: self.differ,
                        triangle_idx: curr_triangle_idx,
                    });
                }

                let next_index_idx = self.differ.triangles[TriangleIdx::get(curr_triangle_idx)][curr_rem].next.unpack();

                if let Some(next_index_idx) = next_index_idx {
                    let (next_triangle_idx, next_rem) = IndexIdx::unflatten(next_index_idx);

                    assert_eq!(
                        self.differ.triangles[TriangleIdx::get(next_triangle_idx)][next_rem].prev.unpack(),
                        Some(index_idx),
                    );
                }

                curr_index_idx = next_index_idx;
            }

            let next_vertex_idx = self.differ.vertices[VertexIdx::get(vertex_idx)].next.unpack();

            if let Some(next_vertex_idx) = next_vertex_idx {
                assert_eq!(
                    self.differ.vertices[VertexIdx::get(next_vertex_idx)].prev.unpack(),
                    VertexOrOuterIdx::Vertex(vertex_idx),
                );
            }

            curr_vertex_idx = next_vertex_idx;
        }

        f.finish()
    }
}

struct AltDebug1Triangle<'a> {
    differ: &'a MeshDiffer,
    triangle_idx: TriangleIdx,
}

impl<'a> Debug for AltDebug1Triangle<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_list();

        for rem in 0..3 {
            let vertex_idx = self.differ.triangles[TriangleIdx::get(self.triangle_idx)][rem].val;
            let vertex = self.differ.vertices[VertexIdx::get(vertex_idx)].val;
            f.entry(&AltDebug1Vertex(vertex));
        }

        f.finish()
    }
}

struct AltDebug1Vertex(Vertex);

impl Debug for AltDebug1Vertex {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&format!(
            "Vertex {{ pos: <{}, {}, {}>, .. }}",
            self.0.pos.x,
            self.0.pos.y,
            self.0.pos.z,
        ))
    }
}


struct AltDebug2<'a>(&'a MeshDiffer);

impl<'a> Debug for AltDebug2<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_struct("MeshDiffer")
            .field("outer", &AltDebug2Outer(self.0))
            .field("vertices", &AltDebug2Vertices(self.0))
            .field("triangles", &AltDebug2Triangles(self.0))
            .field("vertices_holes", &AltDebug2Queue(&self.0.vertices_holes))
            .field("vertices_writes", &AltDebug2Queue(&self.0.vertices_writes))
            .field("triangles_holes", &AltDebug2Queue(&self.0.triangles_holes))
            .field("indices_writes", &AltDebug2Queue(&self.0.indices_writes))
            .finish()
    } 
}

struct AltDebug2Outer<'a>(&'a MeshDiffer);

impl<'a> Debug for AltDebug2Outer<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_map();
        for (key, vertex_idx) in self.0.outer.iter() {
            f.entry(
                &DebugOneLine(OuterIdx(key)),
                &DebugOneLine(vertex_idx.unpack()),
            );
        }
        f.finish()
    }
}

struct AltDebug2Vertices<'a>(&'a MeshDiffer);

impl<'a> Debug for AltDebug2Vertices<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_map();
        for (i, vertex) in self.0.vertices.iter().enumerate() {
            f.entry(
                &DebugOneLine(VertexIdx(i)),
                &AltDebug2VertexElem(vertex),
            );
        }
        f.finish()
    }
}

struct AltDebug2VertexElem<'a>(&'a VertexElem);

impl<'a> Debug for AltDebug2VertexElem<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_struct("VertexElem")
            .field("prev", &DebugOneLine(self.0.prev.unpack()))
            .field("next", &DebugOneLine(self.0.next.unpack()))
            .field("first_index", &DebugOneLine(self.0.first_index.unpack()))
            .field("val", &AltDebug1Vertex(self.0.val))
            .finish()
    }
}

struct AltDebug2Triangles<'a>(&'a MeshDiffer);

impl<'a> Debug for AltDebug2Triangles<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_map();
        for (i, triangle) in self.0.triangles.iter().enumerate() {
            f.entry(
                &DebugOneLine(TriangleIdx(i)),
                &AltDebug2TriangleElem(triangle),
            );
        }
        f.finish()
    }
}

struct AltDebug2TriangleElem<'a>(&'a [IndexElem; 3]);

impl<'a> Debug for AltDebug2TriangleElem<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_list();
        for rem in 0..3 {
            f.entry(&AltDebug2IndexElem(rem, &self.0[rem]));
        }
        f.finish()
    }
}

struct AltDebug2IndexElem<'a>(usize, &'a IndexElem);

impl<'a> Debug for AltDebug2IndexElem<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_struct(&format!("IndexElem @ {:?}", IndexIdx(self.0)))
            .field("prev", &DebugOneLine(self.1.prev.unpack()))
            .field("next", &DebugOneLine(self.1.next.unpack()))
            .field("val", &DebugOneLine(self.1.val))
            .finish()
    }
}

struct AltDebug2Queue<'a, T>(&'a VecDeque<T>);

impl<'a, T: Debug> Debug for AltDebug2Queue<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut f = f.debug_list();
        for item in self.0.iter() {
            f.entry(&DebugOneLine(item));
        }
        f.finish()
    }
}

struct DebugOneLine<T>(T);

impl<T: Debug> Debug for DebugOneLine<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&format!("{:?}", self.0))
    }
}


impl<T: GpuVecElem, I: Iterator<Item=(usize, T)>> GpuVecDiff<I> {
    pub fn patch(self, gpu_vec: &mut GpuVec<T>, renderer: &Renderer) {
        // TODO: this is wasteful

        renderer.set_gpu_vec_len(gpu_vec, self.new_len);

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

        renderer.patch_gpu_vec(gpu_vec, &patches);
    }
}
