//! Type-level dimensional constraints.

use std::fmt::Debug;


/// Dimensional constraint on some axis (X or Y). A type-level tuple of `In`
/// and `Out` associated types, generally never instantiated itself.
///
/// When a `GuiNode` is sized, converting it to a `SizedGuiNode`, `Self::In`
/// is passed as the constraint, and `Self::Out` is returned as remaining size
/// determination within that constraint.
pub trait DimConstraint {
    type In: Debug + Default + Copy;
    type Out: Debug + Default + Copy;

    fn get(i: Self::In, o: Self::Out) -> f32;
}


/// `DimensionalConstraint` type-level variant that the parent sets the child's
/// size in that dimension.
pub enum DimParentSets {}

impl DimConstraint for DimParentSets {
    type In = f32;
    type Out = ();

    fn get(n: f32, (): ()) -> f32 { n }
}


/// `DimensionalConstraint` type-level variant that the child sets its own size
/// in that dimension.
pub enum DimChildSets {}

impl DimConstraint for DimChildSets {
    type In = ();
    type Out = f32;

    fn get((): (), n: f32) -> f32 { n }
}
