//-- flux/mod.rs ------------------------------------------------------------------------------------------------------------------------

pub mod fluxbasics;
pub mod fluxexport;
pub mod instream;
pub mod jsonoutstrm;
pub mod outstream;

pub use instream::{BuffStream, FixedStream, IStream};
pub use jsonoutstrm::JsonOutStream;
pub use outstream::OutStream;

#[cfg(test)]
mod _tests;
pub use fluxexport::{FieldExp, IFluxExportSink, IFluxExportSource};

//---------------------------------------------------------------------------------------------------------------------------------
pub mod fluximport;
pub use fluximport::{FieldImp, IFluxImportSink, IFluxImportSource};
