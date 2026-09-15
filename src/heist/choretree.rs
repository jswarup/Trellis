// choretree.rs ----------------------------------------------------------------------------------------------------

use crate::heist::maestro::Maestro;
use crate::silo::buff::Buff;
use crate::silo::stash::Stash;
use crate::stalks::work::{IWorker, WorkPtr};
use std::ops::{BitOr, Shr};
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// Chore — concrete execution unit in a Heist chore tree.
// Modeled directly from Trellis heist/choretree.h.

pub type ChoreFn = Box<dyn Fn(&mut dyn IWorker) + Send + Sync>;
pub type ChoreSharedFn = Arc<dyn Fn(&mut dyn IWorker) + Send + Sync>;

#[derive(Clone)]
pub struct Chore {
    pub _DocStr: &'static str,
    pub _Closure: Option<fn(&mut dyn IWorker)>,
    pub _Work: Option<ChoreSharedFn>,
}

impl Chore {
    pub const fn New(f: fn(&mut dyn IWorker)) -> Self {
        Self {
            _DocStr: "",
            _Closure: Some(f),
            _Work: None,
        }
    }

    pub const fn WithDoc(doc_str: &'static str, f: fn(&mut dyn IWorker)) -> Self {
        Self {
            _DocStr: doc_str,
            _Closure: Some(f),
            _Work: None,
        }
    }

    pub fn FromClosure<F>(doc_str: &'static str, f: F) -> Self
    where
        F: Fn(&mut dyn IWorker) + Send + Sync + 'static,
    {
        Self {
            _DocStr: doc_str,
            _Closure: None,
            _Work: Some(Arc::new(f)),
        }
    }

    pub fn FromFn(f: fn(&mut dyn IWorker)) -> Self {
        Self::New(f)
    }

    pub fn DocStr(&self) -> &'static str {
        self._DocStr
    }

    pub fn Post(&self, maestro: &Maestro, tails: &mut Stash<u16>) -> u16 {
        let work = if let Some(closure) = self._Closure {
            WorkPtr::FromFn(closure)
        } else if let Some(ref work_arc) = self._Work {
            let work_clone = work_arc.clone();
            WorkPtr::FromClosure(move |w| work_clone(w))
        } else {
            WorkPtr::Null()
        };

        let job_id = maestro.ConstructJob(0, work);
        tails.PushBack(job_id);
        job_id
    }

    pub fn Then(self, other: impl Into<ChoreNode>) -> ChoreNode {
        ChoreNode::Leaf(self).Then(other)
    }

    pub fn Par(self, other: impl Into<ChoreNode>) -> ChoreNode {
        ChoreNode::Leaf(self).Par(other)
    }
}

//-------------------------------------------------------------------------------------------------
// ChoreNode — DAG node representing sequential (< or >>) or parallel (|) composition.

#[derive(Clone)]
pub enum ChoreNode {
    Leaf(Chore),
    Seq(Box<ChoreNode>, Box<ChoreNode>),
    Par(Box<ChoreNode>, Box<ChoreNode>),
}

impl ChoreNode {
    pub fn Then(self, other: impl Into<ChoreNode>) -> Self {
        ChoreNode::Seq(Box::new(self), Box::new(other.into()))
    }

    pub fn Par(self, other: impl Into<ChoreNode>) -> Self {
        ChoreNode::Par(Box::new(self), Box::new(other.into()))
    }
}

impl From<Chore> for ChoreNode {
    fn from(chore: Chore) -> Self {
        ChoreNode::Leaf(chore)
    }
}

// Operator overloads: `>>` for sequential, `|` for parallel composition

impl BitOr<Chore> for Chore {
    type Output = ChoreNode;
    fn bitor(self, rhs: Chore) -> ChoreNode {
        ChoreNode::Par(
            Box::new(ChoreNode::Leaf(self)),
            Box::new(ChoreNode::Leaf(rhs)),
        )
    }
}

impl BitOr<ChoreNode> for Chore {
    type Output = ChoreNode;
    fn bitor(self, rhs: ChoreNode) -> ChoreNode {
        ChoreNode::Par(Box::new(ChoreNode::Leaf(self)), Box::new(rhs))
    }
}

impl BitOr<Chore> for ChoreNode {
    type Output = ChoreNode;
    fn bitor(self, rhs: Chore) -> ChoreNode {
        ChoreNode::Par(Box::new(self), Box::new(ChoreNode::Leaf(rhs)))
    }
}

impl BitOr<ChoreNode> for ChoreNode {
    type Output = ChoreNode;
    fn bitor(self, rhs: ChoreNode) -> ChoreNode {
        ChoreNode::Par(Box::new(self), Box::new(rhs))
    }
}

impl Shr<Chore> for Chore {
    type Output = ChoreNode;
    fn shr(self, rhs: Chore) -> ChoreNode {
        ChoreNode::Seq(
            Box::new(ChoreNode::Leaf(self)),
            Box::new(ChoreNode::Leaf(rhs)),
        )
    }
}

impl Shr<ChoreNode> for Chore {
    type Output = ChoreNode;
    fn shr(self, rhs: ChoreNode) -> ChoreNode {
        ChoreNode::Seq(Box::new(ChoreNode::Leaf(self)), Box::new(rhs))
    }
}

impl Shr<Chore> for ChoreNode {
    type Output = ChoreNode;
    fn shr(self, rhs: Chore) -> ChoreNode {
        ChoreNode::Seq(Box::new(self), Box::new(ChoreNode::Leaf(rhs)))
    }
}

impl Shr<ChoreNode> for ChoreNode {
    type Output = ChoreNode;
    fn shr(self, rhs: ChoreNode) -> ChoreNode {
        ChoreNode::Seq(Box::new(self), Box::new(rhs))
    }
}

//-------------------------------------------------------------------------------------------------
// PostChoreNode — recursively posts a ChoreNode into Maestro/Atelier execution graph.

pub fn PostChoreNode(node: &ChoreNode, maestro: &Maestro, tails: &mut Stash<u16>) -> u16 {
    match node {
        ChoreNode::Leaf(chore) => chore.Post(maestro, tails),
        ChoreNode::Seq(left, right) => {
            let mut left_tails = Stash::WithCapacity(64);
            let head_l = PostChoreNode(left, maestro, &mut left_tails);
            let head_r = PostChoreNode(right, maestro, tails);

            if let Some(state) = maestro.State() {
                while let Some(left_tail) = left_tails.Pop() {
                    state.SetSucc(left_tail, head_r);
                }
            }
            head_l
        }
        ChoreNode::Par(left, right) => {
            let mut left_tails = Stash::WithCapacity(64);
            let mut right_tails = Stash::WithCapacity(64);
            let head_l = PostChoreNode(left, maestro, &mut left_tails);
            let head_r = PostChoreNode(right, maestro, &mut right_tails);

            while let Some(t) = left_tails.Pop() {
                tails.PushBack(t);
            }
            while let Some(t) = right_tails.Pop() {
                tails.PushBack(t);
            }

            let heads = Buff::FromDispenser(2, |i| if i == 0 { head_l } else { head_r });
            maestro.ConstructEnqueArr(0, heads)
        }
    }
}
