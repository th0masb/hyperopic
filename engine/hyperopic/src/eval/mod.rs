mod castling;
pub mod material;
mod pawns;
mod safety;
mod space;
pub mod tables;

pub use castling::CastlingFacet;
pub use pawns::PawnStructureFacet;
pub use safety::SafetyFacet;
pub use space::SpaceFacet;
pub use tables::PieceSquareTablesFacet;

// Add facets for:
// - Pins/xrays
// - Knight outposts
