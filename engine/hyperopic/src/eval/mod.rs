mod castling;
mod development;
mod knightrim;
pub mod material;
mod pawns;
mod safety;
mod space;
pub mod tables;

pub use castling::CastlingFacet;
pub use development::DevelopmentFacet;
pub use knightrim::KnightRimFacet;
pub use pawns::PawnStructureFacet;
pub use safety::SafetyFacet;
pub use space::SpaceFacet;
pub use tables::PieceSquareTablesFacet;

// Add facets for:
// - Pins/xrays
// - Knight outposts
