// dset.rs ---------------------------------------------------------------------------------------------------------
use crate::silo::seg::USeg;
use crate::silo::stash::Stash;

//-------------------------------------------------------------------------------------------------

// DisjointSet — union-find with path compression and union-by-rank.
// Modeled directly from Trellis silo/dset.h and Kosh silo/disjoint_set.rs.
// Uses Stash for dynamic growth and zero-virtual contiguous storage.
pub struct DisjointSet
{
    pub _Parent: Stash< u32>,
    pub _Rank: Stash< u8>,
}
impl DisjointSet
{

    //---------------------------------------------------------------------------------------------

    // Constructors & Factories
    pub const fn  New() -> Self
    {
        Self {
            _Parent: Stash::New(),
            _Rank: Stash::New(),
        }
    }
    pub fn  WithCapacity( capacity: u32) -> Self
    {
        let  mut dset = Self {
            _Parent: Stash::WithCapacity( capacity),
            _Rank: Stash::WithCapacity( capacity),
        };
        USeg::FromLen( capacity).Traverse( |i| {
            dset._Parent.Push( i);
            dset._Rank.Push( 0);
        });
        dset
    }

    //---------------------------------------------------------------------------------------------

    // Accessors & Queries
    #[inline]
    pub fn  Size( &self) -> u32
    {
        self._Parent.Size()
    }
    pub fn  FindConst( &self, elem: u32) -> u32
    {
        assert!( elem < self.Size(), "DisjointSet element out of bounds");
        let  mut curr = elem;
        while self._Parent[curr] != curr
        {
            curr = self._Parent[curr];
        }
        curr
    }
    pub fn  Find( &mut self, elem: u32) -> u32
    {
        assert!( elem < self.Size(), "DisjointSet element out of bounds");
        let  mut curr = elem;
        while self._Parent[curr] != curr
        {
            curr = self._Parent[curr];
        }
        let  root = curr;
        // Path compression
        let  mut node = elem;
        while self._Parent[node] != node
        {
            let  next = self._Parent[node];
            self._Parent[node] = root;
            node = next;
        }
        root
    }
    pub fn  Union( &mut self, a: u32, b: u32) -> u32
    {
        let  root_a = self.Find( a);
        let  root_b = self.Find( b);
        if root_a == root_b
        {
            return root_a;
        }
        let  rank_a = self._Rank[root_a];
        let  rank_b = self._Rank[root_b];
        if rank_a < rank_b
        {
            self._Parent[root_a] = root_b;
            root_b
        } else if rank_a > rank_b
        {
            self._Parent[root_b] = root_a;
            root_a
        } else
        {
            self._Parent[root_b] = root_a;
            self._Rank[root_a] += 1;
            root_a
        }
    }
    pub fn  Same( &mut self, a: u32, b: u32) -> bool
    {
        self.Find( a) == self.Find( b)
    }
    pub fn  Grow( &mut self, count: u32)
    {
        let  start = self.Size();
        USeg::WithLen( start, count).Traverse( |idx| {
            self._Parent.Push( idx);
            self._Rank.Push( 0);
        });
    }
}
impl Default for DisjointSet
{
    fn  default() -> Self
    {
        Self::New()
    }
}
