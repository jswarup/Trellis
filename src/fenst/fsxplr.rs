//-- fsxplr.rs -------------------------------------------------------------------------------------------------------
use	crate::fenst::{ BranchXplr, LeafXplr, StreamChunk, Xplr };
use	crate::silo::{ Buff, Stash };
use	std::fs;
use	std::path::PathBuf;
const K_MAX_CHUNK_BYTES: u32 = 1024 * 1024;

//---------------------------------------------------------------------------------------------------------------------------------

fn	PathName( path: &str) -> String
{
    PathBuf::from( path)
        .file_name()
        .map( |name| name.to_string_lossy().into_owned())
        .unwrap_or_else( || path.to_string())
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct FsLeaf
{
    _Name: String,
    _Path: String,
    _Extension: String,
}
impl FsLeaf
{
    pub fn	New( path: String) -> Self
    {
        let  	extension = PathBuf::from( &path)
            .extension()
            .map( |value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            _Name: PathName( &path),
            _Path: path,
            _Extension: extension,
        }
    }
}
impl Xplr for FsLeaf {
    fn	Name( &self) -> &str
    {
        &self._Name
    }
    fn	Path( &self) -> &str
    {
        &self._Path
    }
    fn	Leaf( &self) -> Option< &dyn LeafXplr>
    {
        Some( self)
    }
}
impl LeafXplr for FsLeaf {
    fn	Size( &self) -> u64
    {
        fs::metadata( &self._Path)
            .map( |metadata| metadata.len())
            .unwrap_or( 0)
    }
    fn	Extension( &self) -> &str
    {
        &self._Extension
    }
    fn	ReadChunk( &self, offset: u64, length: u32) -> Result< StreamChunk, String>
    {
        use std::io::{ Read, Seek, SeekFrom };
        if length > K_MAX_CHUNK_BYTES {
            return Err( format!( "Chunk length exceeds {} bytes", K_MAX_CHUNK_BYTES));
        }
        let mut file = fs::File::open( &self._Path).map_err( |error| error.to_string())?;
        let total_size = file.metadata().map( |m| m.len()).unwrap_or( 0);
        file.seek( SeekFrom::Start( offset)).map_err( |error| error.to_string())?;
        let mut buffer = vec![0u8; length.min( K_MAX_CHUNK_BYTES) as usize];
        let bytes_read = (&mut file).take( length as u64).read( &mut buffer).map_err( |error| error.to_string())?;
        buffer.truncate( bytes_read);
        let content = String::from_utf8_lossy( &buffer).into_owned();
        Ok( StreamChunk::New( 
            self._Path.clone(),
            offset,
            total_size,
            content,
        ))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct FsBranch
{
    _Name: String,
    _Path: String,
}
impl FsBranch
{
    pub fn	New( path: String) -> Self
    {
        Self {
            _Name: PathName( &path),
            _Path: path,
        }
    }
}
impl Xplr for FsBranch {
    fn	Name( &self) -> &str
    {
        &self._Name
    }
    fn	Path( &self) -> &str
    {
        &self._Path
    }
    fn	Branch( &self) -> Option< &dyn BranchXplr>
    {
        Some( self)
    }
}
impl BranchXplr for FsBranch {
    fn	Children( &self) -> Result< Buff< Box< dyn Xplr>>, String>
    {
        let  	entries = fs::read_dir( &self._Path).map_err( |error| error.to_string())?;
        let  	mut dirs: Stash< Box< dyn Xplr>> = Stash::New();
        let  	mut files: Stash< Box< dyn Xplr>> = Stash::New();
        for entry in entries.flatten() {
            let  	path = entry.path();
            let  	name = entry.file_name();
            if name.to_string_lossy().starts_with( '.') {
                continue;
            }
            let is_dir = entry.file_type().map( |ft| ft.is_dir()).unwrap_or_else( |_| path.is_dir());
            if is_dir {
                dirs.Push( Box::new( FsBranch::New( path.to_string_lossy().into_owned())));
            } else {
                files.Push( Box::new( FsLeaf::New( path.to_string_lossy().into_owned())));
            }
        }
        dirs.MutArr().QSort( |a, b| a.Name() < b.Name());
        files.MutArr().QSort( |a, b| a.Name() > b.Name());
        while let  	Some( file) = files.Pop() {
            dirs.Push( file);
        }
        Ok( dirs.IntoBuff())
    }
    fn	ChildCount( &self) -> Result< u32, String>
    {
        let  	children = self.Children()?;
        Ok( children.Size())
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
/// Represents a drive entry on Windows (e.g. `C:`, `D:`) whose children are the files and folders on that drive.
pub struct FsDriveBranch
{
    _Name: String,
    _Path: String,
}
impl FsDriveBranch
{
    pub fn	New( name: String, path: String) -> Self
    {
        Self {
            _Name: name,
            _Path: path,
        }
    }
}
impl Xplr for FsDriveBranch {
    fn	Name( &self) -> &str
    {
        &self._Name
    }
    fn	Path( &self) -> &str
    {
        &self._Path
    }
    fn	Branch( &self) -> Option< &dyn BranchXplr>
    {
        Some( self)
    }
}
impl BranchXplr for FsDriveBranch {
    fn	Children( &self) -> Result< Buff< Box< dyn Xplr>>, String>
    {
        FsBranch::New( self._Path.clone()).Children()
    }
    fn	ChildCount( &self) -> Result< u32, String>
    {
        FsBranch::New( self._Path.clone()).ChildCount()
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
/// The root Filesystem branch (`file://`).
/// On Windows, its first-level children are the detected system drives (`C:`, `D:`, etc.).
/// On non-Windows platforms, its children are the root directories under `/`.
pub struct FsRootsBranch
{
    _Name: String,
    _Path: String,
}
impl FsRootsBranch
{
    pub fn	New() -> Self
    {
        Self {
            _Name: "Filesystem".to_string(),
            _Path: "file://".to_string(),
        }
    }

    pub fn	DetectDrives() -> Vec< String>
    {
        #[cfg( target_os = "windows")] {
            let  	mut drives = Vec::new();
            for letter in b'A'..=b'Z' {
                let  	drive_str = format!( "{}:\\", letter as char);
                if std::path::Path::new( &drive_str).exists() {
                    drives.push( format!( "{}:", letter as char));
                }
            }
            if drives.is_empty() {
                drives.push( "C:".to_string());
            }
            drives
        }
        #[cfg( not( target_os = "windows"))] {
            vec!["/".to_string()]
        }
    }
}
impl Default for FsRootsBranch {
    fn	default() -> Self
    {
        Self::New()
    }
}
impl Xplr for FsRootsBranch {
    fn	Name( &self) -> &str
    {
        &self._Name
    }
    fn	Path( &self) -> &str
    {
        &self._Path
    }
    fn	Branch( &self) -> Option< &dyn BranchXplr>
    {
        Some( self)
    }
}
impl BranchXplr for FsRootsBranch {
    fn	Children( &self) -> Result< Buff< Box< dyn Xplr>>, String>
    {
        let  	drives = Self::DetectDrives();
        let  	mut stash = Stash::New();
        for drive in drives {
            #[cfg( target_os = "windows")] {
                let  	path = format!( "{}\\", drive);
                stash.Push( Box::new( FsDriveBranch::New( drive, path)) as Box< dyn Xplr>);
            }
            #[cfg( not( target_os = "windows"))] {
                stash.Push( Box::new( FsBranch::New( drive)) as Box< dyn Xplr>);
            }
        }
        Ok( stash.IntoBuff())
    }
    fn	ChildCount( &self) -> Result< u32, String>
    {
        Ok( Self::DetectDrives().len() as u32)
    }
}
