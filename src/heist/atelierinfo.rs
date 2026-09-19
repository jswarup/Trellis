use	crate::heist::atelier::AtelierState;
use	crate::silo::arr::Arr;
use	crate::silo::stash::Stash;
use	std::collections::HashSet;
use	std::fmt;
use	std::sync::atomic::Ordering;
#[derive( Clone, Copy, Default)]
pub struct JobInfo
{
    pub _JobId: u16,
    pub _SuccId: u16,
    pub _SzPred: u16,
}
impl JobInfo
{
    pub fn	New( state: &AtelierState, job_id: u16) -> Self
    {
        let  	succ_id = state._SuccIds[job_id as u32].load( Ordering::SeqCst);
        let  	sz_pred = state._SzPreds[job_id as u32].load( Ordering::SeqCst);
        Self {
            _JobId: job_id,
            _SuccId: succ_id,
            _SzPred: sz_pred,
        }
    }
}
pub struct AtelierInfo
{
    pub _HookedStash: Stash< JobInfo>,
}
impl AtelierInfo
{
    pub fn	FetchConnectedJobs( 
        state: &AtelierState,
        job_ids: Arr< '_, u16>,
        job_stash: &mut Stash< JobInfo>,
    )
    {
        let  	mut job_set = HashSet::< u16>::new();
        let  	mut process_stash = Stash::< u16>::WithCapacity( 1024);
        for i in 0..job_ids.Size() {
            process_stash.Push( job_ids[i]);
        }
        while process_stash.Size() > 0 {
            let  	job_id = process_stash.Pop().unwrap();
            if job_set.insert( job_id) {
                let  	succ_id = state._SuccIds[job_id as u32].load( Ordering::SeqCst);
                if succ_id != 0 {
                    process_stash.Push( succ_id);
                }
                job_stash.Push( JobInfo::New( state, job_id));
            }
        }
    }
    pub fn	TraceJobs( state: &AtelierState) -> AtelierInfo
    {
        let  	mut info = AtelierInfo {
            _HookedStash: Stash::WithCapacity( 1024),
        };
        for i in 0..state._Maestros.Cap() {
            let  	maestro = &state._Maestros[i];
            let  	q = maestro._RunQueue.Lock();
            Self::FetchConnectedJobs( state, q.AsArr(), &mut info._HookedStash);
        }
        info
    }
}
impl fmt::Display for JobInfo {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( 
            f,
            "{{ JobId: {}, Succ: {}, Pred: {}}}",
            self._JobId, self._SuccId, self._SzPred
        )
    }
}
impl fmt::Display for AtelierInfo {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "Atel[ Hooked:")?;
        let  	arr = self._HookedStash.AsArr();
        for i in 0..arr.Size() {
            write!( f, " {}", arr[i])?;
        }
        write!( f, "] ")
    }
}
