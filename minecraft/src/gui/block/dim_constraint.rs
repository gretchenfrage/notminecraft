//! Type-level dimensional constraints.


/// Dimensional constraint on some axis (X or Y). A type-level tuple of `In`
/// and `Out` associated types, generally never instantiated itself.
///
/// When a `GuiNode` is sized, converting it to a `SizedGuiNode`, `Self::In`
/// is passed as the constraint, and `Self::Out` is returned as remaining size
/// determination within that constraint.
pub trait DimConstraint {
    type In;
    type Out;
}


/// `DimensionalConstraint` type-level variant that the parent sets the child's
/// size in that dimension.
pub enum DimParentSets {}

impl DimConstraint for DimParentSets {
    type In = f32;
    type Out = ();
}


/// `DimensionalConstraint` type-level variant that the child sets its own size
/// in that dimension.
pub enum DimChildSets {}

impl DimConstraint for DimChildSets {
    type In = ();
    type Out = f32;
}
