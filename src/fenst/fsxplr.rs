//-- fsxplr.rs -------------------------------------------------------------------------------------------------------

use crate::fenst::{BranchXplr, LeafXplr, StreamChunk, Xplr};
use crate::silo::{Buff, Stash};
use std::fs;
use std::path::PathBuf;

const K_MAX_CHUNK_BYTES: u32 = 1024 * 1024;

//---------------------------------------------------------------------------------------------------------------------------------

fn PathName(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct FsLeaf {
    _Name: String,
    _Path: String,
    _Extension: String,
}

impl FsLeaf {
    pub fn New(path: String) -> Self {
        let extension = PathBuf::from(&path)
            .extension()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self { _Name: PathName(&path), _Path: path, _Extension: extension }
    }
}

impl Xplr for FsLeaf {
    fn Name(&self) -> &str { &self._Name }
    fn Path(&self) -> &str { &self._Path }
    fn AsLeaf(&self) -> Option<&dyn LeafXplr> { Some(self) }
}

impl LeafXplr for FsLeaf {
    fn Size(&self) -> u64 { fs::metadata(&self._Path).map(|metadata| metadata.len()).unwrap_or(0) }
    fn Extension(&self) -> &str { &self._Extension }

    fn ReadChunk(&self, offset: u64, length: u32) -> Result<StreamChunk, String> {
        if length > K_MAX_CHUNK_BYTES {
            return Err(format!("Chunk length exceeds {} bytes", K_MAX_CHUNK_BYTES));
        }
        let bytes = fs::read(&self._Path).map_err(|error| error.to_string())?;
        let totalSize = bytes.len() as u64;
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(bytes.len());
        let end = start.saturating_add(length as usize).min(bytes.len());
        let content = String::from_utf8_lossy(&bytes[start..end]).into_owned();
        Ok(StreamChunk::New(self._Path.clone(), offset, totalSize, content))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct FsBranch {
    _Name: String,
    _Path: String,
}

impl FsBranch {
    pub fn New(path: String) -> Self { Self { _Name: PathName(&path), _Path: path } }
}

impl Xplr for FsBranch {
    fn Name(&self) -> &str { &self._Name }
    fn Path(&self) -> &str { &self._Path }
    fn AsBranch(&self) -> Option<&dyn BranchXplr> { Some(self) }
}

impl BranchXplr for FsBranch {
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String> {
        let entries = fs::read_dir(&self._Path).map_err(|error| error.to_string())?;
        let mut dirs = Stash::New();
        let mut files = Stash::New();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let item: Box<dyn Xplr> = if path.is_dir() {
                Box::new(FsBranch::New(path.to_string_lossy().into_owned()))
            } else {
                Box::new(FsLeaf::New(path.to_string_lossy().into_owned()))
            };
            if item.IsLeaf() {
                files.Push(item);
            } else {
                dirs.Push(item);
            }
        }
        while let Some(file) = files.Pop() {
            dirs.Push(file);
        }
        Ok(dirs.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String> {
        let children = self.Children()?;
        Ok(children.Size())
    }
}
