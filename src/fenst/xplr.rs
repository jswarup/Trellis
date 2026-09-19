//-- xplr.rs ---------------------------------------------------------------------------------------------------------
use	crate::silo::Buff;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, PartialEq, Eq)]
pub struct XplrNodeInfo
{
    _Id: String,
    _Name: String,
    _IsLeaf: bool,
    _Provider: String,
    _Size: u64,
    _Extension: String,
}
impl XplrNodeInfo
{
    pub fn	New( 
        id: String, name: String, isLeaf: bool, provider: String, size: u64, extension: String,
    ) -> Self
    {
        Self {
            _Id: id,
            _Name: name,
            _IsLeaf: isLeaf,
            _Provider: provider,
            _Size: size,
            _Extension: extension,
        }
    }
    pub fn	Id( &self) -> &str
    {
        &self._Id
    }
    pub fn	Name( &self) -> &str
    {
        &self._Name
    }
    pub fn	IsLeaf( &self) -> bool
    {
        self._IsLeaf
    }
    pub fn	Provider( &self) -> &str
    {
        &self._Provider
    }
    pub fn	Size( &self) -> u64
    {
        self._Size
    }
    pub fn	Extension( &self) -> &str
    {
        &self._Extension
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, PartialEq, Eq)]
pub struct StreamChunk
{
    _Path: String,
    _Offset: u64,
    _TotalSize: u64,
    _Content: String,
}
impl StreamChunk
{
    pub fn	New( path: String, offset: u64, totalSize: u64, content: String) -> Self
    {
        Self {
            _Path: path,
            _Offset: offset,
            _TotalSize: totalSize,
            _Content: content,
        }
    }
    pub fn	Path( &self) -> &str
    {
        &self._Path
    }
    pub fn	Offset( &self) -> u64
    {
        self._Offset
    }
    pub fn	TotalSize( &self) -> u64
    {
        self._TotalSize
    }
    pub fn	Content( &self) -> &str
    {
        &self._Content
    }
    pub fn	Length( &self) -> u32
    {
        self._Content.len() as u32
    }
    pub fn	IsEof( &self) -> bool
    {
        self._Offset + self._Content.len() as u64 >= self._TotalSize
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait Xplr {
    fn	Name( &self) -> &str;
    fn	Path( &self) -> &str;
    fn	IsLeaf( &self) -> bool
    {
        self.AsLeaf().is_some()
    }
    fn	AsLeaf( &self) -> Option< &dyn LeafXplr>
    {
        None
    }
    fn	AsBranch( &self) -> Option< &dyn BranchXplr>
    {
        None
    }
    fn	ToInfo( &self, provider: &str) -> XplrNodeInfo
    {
        let  	size = self.AsLeaf().map( |leaf| leaf.Size()).unwrap_or( 0);
        let  	extension = self
            .AsLeaf()
            .map( |leaf| leaf.Extension().to_string())
            .unwrap_or_default();
        XplrNodeInfo::New( 
            self.Path().to_string(),
            self.Name().to_string(),
            self.IsLeaf(),
            provider.to_string(),
            size,
            extension,
        )
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait LeafXplr: Xplr {
    fn	Size( &self) -> u64;
    fn	Extension( &self) -> &str;
    fn	ReadChunk( &self, offset: u64, length: u32) -> Result< StreamChunk, String>;
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait BranchXplr: Xplr {
    fn	Children( &self) -> Result< Buff< Box< dyn Xplr>>, String>;
    fn	ChildCount( &self) -> Result< u32, String>;
}
