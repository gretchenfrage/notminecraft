//! GUI system.
//!
//! I should explain this better, but:
//!
//! - there is a stack of `GuiStateFrame`
//! - whenever most things happen, like an input event or it's time to draw,
//!   the top of the stack is responsible for that
//!
//! A `GuiStateFrame` has certain "global" state and behavior, but also tends
//! to have various semi-independent sub-entanglements of state and behavior
//! which can each be meaningfully "positioned"--each given some transform and
//! then layered one after the other. The `GuiNode` trait is this unit of
//! positionable behavior.
//!
//! Generally a `GuiStateFrame` will use the `block` system to handle the
//! laying out of its `GuiNode`s. Basically the process is:
//!
//! 1. A `GuiStateFrame` is borrowed as a tree of `GuiBlock`s
//! 2. Sizing occurs, recursively converting the `GuiBlock` tree into a
//!    `SizedGuiBlock` tree. Dimensional constraints are passed on windup and
//!    decisions within those constraints are returned on winddown. Positioning
//!    is not yet knowable at the sizing phase.
//! 3. Positioning occurs. The `GuiBlock` tree is recursively flattened out
//!    into a sequence of positioned `GuiNode`s with a `GuiVisitor`, which uses
//!    a `graphics`-like stack instruction system.
//! 4. Interleaved with the previous step, each node once positioned has the
//!    relevant handler (for drawing, rendering, etc.) invoked. 
//!
//! All of this is fused together with monomorphization to make it very fast
//! and not require any allocations.


mod context;
mod event;
mod state_frame;
mod state_frame_obj;
mod node;
mod block;
mod gui_event_loop;

pub mod blocks;

pub use self::{
    context::{
        GuiGlobalContext,
        GuiSpatialContext,
        GuiWindowContext,
        FocusLevel,
    },
    event::ScrolledAmount,
    state_frame::GuiStateFrame,
    state_frame_obj::GuiStateFrameObj,
    node::{
        GuiNode,
        GuiVisitorTarget,
        GuiVisitor,
    },
    block::{
        dim_constraint::{
            DimConstraint,
            DimParentSets,
            DimChildSets,
        },
        gui_block::{
            GuiBlock,
            SizedGuiBlock,
        },
        gui_block_seq::{
            GuiBlockSeq,
            SizedGuiBlockSeq,
            GuiVisitorMaperator,
        },
        sized_gui_block_seq_flatten::SizedGuiBlockFlatten,
    },
    gui_event_loop::GuiEventLoop,
};
