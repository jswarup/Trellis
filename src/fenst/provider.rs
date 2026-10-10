//-- provider.rs -----------------------------------------------------------------------------------------------------
use	crate::fenst::{ BranchXplr, FsBranch };
use	crate::silo::{ Buff, IArr, Stash };

//---------------------------------------------------------------------------------------------------------------------------------

pub trait XplrProvider: Send + Sync {
    fn	Scheme( &self) -> &str;
    fn	OpenRoot( &self, uri: &str) -> Result< Box< dyn BranchXplr>, String>;
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct FsProvider;
impl FsProvider
{
    pub fn	New() -> Self
    {
        Self
    }
}
impl XplrProvider for FsProvider {
    fn	Scheme( &self) -> &str
    {
        "file"
    }
    fn	OpenRoot( &self, uri: &str) -> Result< Box< dyn BranchXplr>, String>
    {
        let  	mut path = uri.strip_prefix( "file://").unwrap_or( uri);
        #[cfg( target_os = "windows")]
        if path.starts_with( '/') && path.len() >= 3 && path.as_bytes()[2] == b':' {
            path = &path[1..];
        }
        if path.is_empty() || path == "/" {
            return Ok( Box::new( crate::fenst::FsRootsBranch::New()));
        }
        Ok( Box::new( FsBranch::New( path.to_string())))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct XplrRegistry
{
    _Providers: Stash< Box< dyn XplrProvider>>,
}
impl XplrRegistry
{
    pub fn	New() -> Self
    {
        let  	mut providers = Stash::New();
        providers.Push( Box::new( FsProvider::New()) as Box< dyn XplrProvider>);
        providers.Push( Box::new( crate::fenst::SettingsProvider::New()) as Box< dyn XplrProvider>);
        providers.Push( Box::new( crate::fenst::ExampleProvider::New()) as Box< dyn XplrProvider>);
        Self {
            _Providers: providers,
        }
    }
    pub fn	Register( &mut self, provider: Box< dyn XplrProvider>) -> bool
    {
        let  	scheme = provider.Scheme();
        if self.HasProvider( scheme) {
            return false;
        }
        self._Providers.Push( provider);
        true
    }
    fn	HasProvider( &self, scheme: &str) -> bool
    {
        let  	mut found = false;
        self._Providers.Arr().Traverse( |provider| {
            if provider.Scheme() == scheme {
                found = true;
            }
        });
        found
    }
    pub fn	Schemes( &self) -> Buff< String>
    {
        let  	mut schemes = Stash::New();
        self._Providers
            .Arr()
            .Traverse( |provider| schemes.Push( provider.Scheme().to_string()));
        schemes.IntoBuff()
    }
    pub fn	OpenRoot( &self, uri: &str) -> Result< ( String, Box< dyn BranchXplr>), String>
    {
        let  	scheme = uri
            .split_once( "://")
            .map( |( scheme, _)| scheme)
            .unwrap_or( "file");
        let  	mut result = None;
        self._Providers.Arr().Traverse( |provider| {
            if result.is_none() && provider.Scheme() == scheme {
                result = Some( provider.OpenRoot( uri));
            }
        });
        let  	root =
            result.ok_or_else( || format!( "No explorer provider for scheme: {}", scheme))??;
        Ok( ( scheme.to_string(), root))
    }
}
impl Default for XplrRegistry {
    fn	default() -> Self
    {
        Self::New()
    }
}
