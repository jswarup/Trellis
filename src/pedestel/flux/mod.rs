//-- flux/mod.rs ------------------------------------------------------------------------------------------------------------------------
pub mod fluxbasics;
pub mod fluxexport;
pub mod instream;
pub mod jsonoutstrm;
pub mod outstream;
pub use	instream::{ BuffStream, FixedStream, IStream };
pub use	jsonoutstrm::JsonOutStream;
pub use	outstream::OutStream;
#[cfg( feature = "tests")]
pub mod _tests;
pub use	fluxexport::{ FieldExp, IFluxExportSink, IFluxExportSource };

//---------------------------------------------------------------------------------------------------------------------------------

pub mod compress;
pub mod file;
pub mod fluximport;
pub use compress::{
    ChunkedStreamReader, ChunkedStreamWriter, CompressBlock, DecompressBlock, FlxCompressKind,
};
pub use file::{
    FluxFile, FluxFileError, FluxFileInfo, FluxFileFormat, FluxFileMetadata, FluxIntegrityStatus,
    FluxReadLimits, FluxValue,
};
pub use fluximport::{
    FieldImp, FluxError, IArrayImport, IFluxImportSink, IFluxImportSource, IObjectImport,
};
