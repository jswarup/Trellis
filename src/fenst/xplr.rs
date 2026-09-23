//-- xplr.rs ---------------------------------------------------------------------------------------------------------
use crate::silo::Buff;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XplrNodeInfo
{
    _Id:        String,
    _Name:      String,
    _IsLeaf:    bool,
    _Provider:  String,
    _Size:      u64,
    _Extension: String,
}
impl XplrNodeInfo
{
    pub fn New(id: String, name: String, isLeaf: bool, provider: String, size: u64,
               extension: String)
               -> Self
    {
        Self { _Id:        id,
               _Name:      name,
               _IsLeaf:    isLeaf,
               _Provider:  provider,
               _Size:      size,
               _Extension: extension, }
    }
    pub fn Id(&self) -> &str { &self._Id }
    pub fn Name(&self) -> &str { &self._Name }
    pub fn IsLeaf(&self) -> bool { self._IsLeaf }
    pub fn Provider(&self) -> &str { &self._Provider }
    pub fn Size(&self) -> u64 { self._Size }
    pub fn Extension(&self) -> &str { &self._Extension }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamChunk
{
    _Path:      String,
    _Offset:    u64,
    _TotalSize: u64,
    _Content:   String,
}
impl StreamChunk
{
    pub fn New(path: String, offset: u64, totalSize: u64, content: String) -> Self
    {
        Self { _Path:      path,
               _Offset:    offset,
               _TotalSize: totalSize,
               _Content:   content, }
    }
    pub fn Path(&self) -> &str { &self._Path }
    pub fn Offset(&self) -> u64 { self._Offset }
    pub fn TotalSize(&self) -> u64 { self._TotalSize }
    pub fn Content(&self) -> &str { &self._Content }
    pub fn Length(&self) -> u32 { self._Content.len() as u32 }
    pub fn IsEof(&self) -> bool { self._Offset + self._Content.len() as u64 >= self._TotalSize }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait Xplr
{
    fn Name(&self) -> &str;
    fn Path(&self) -> &str;
    fn IsLeaf(&self) -> bool { self.Leaf().is_some() }
    fn Leaf(&self) -> Option<&dyn LeafXplr> { None }
    fn Branch(&self) -> Option<&dyn BranchXplr> { None }
    fn ToInfo(&self, provider: &str) -> XplrNodeInfo
    {
        let size = self.Leaf().map(|leaf| leaf.Size()).unwrap_or(0);
        let extension = self.Leaf()
                            .map(|leaf| leaf.Extension().to_string())
                            .unwrap_or_default();
        XplrNodeInfo::New(self.Path().to_string(),
                          self.Name().to_string(),
                          self.IsLeaf(),
                          provider.to_string(),
                          size,
                          extension)
    }
    fn TraverseDepth<F>(&self, lambda: F)
        where Self: Sized,
              F: FnMut(&[&dyn Xplr], bool) -> bool
    {
        TraverseDepth(self, lambda);
    }
}
impl dyn Xplr + '_
{
    pub fn TraverseDepth<F>(&self, lambda: F)
        where F: FnMut(&[&dyn Xplr], bool) -> bool
    {
        TraverseDepth(self, lambda);
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait LeafXplr: Xplr
{
    fn Size(&self) -> u64;
    fn Extension(&self) -> &str;
    fn ReadChunk(&self, offset: u64, length: u32) -> Result<StreamChunk, String>;
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait BranchXplr: Xplr
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>;
    fn ChildCount(&self) -> Result<u32, String>;
}

//---------------------------------------------------------------------------------------------------------------------------------

struct ChildCursor
{
    _Children: Option<Buff<Box<dyn Xplr>>>,
    _Index:    u32,
}
impl ChildCursor
{
    fn New() -> Self
    {
        Self { _Children: None,
               _Index:    0, }
    }
    fn IsNull(&self) -> bool { self._Children.is_none() }
    fn Size(&self) -> u32
    {
        match &self._Children {
            Some(buff) => buff.Len().saturating_sub(self._Index),
            None => 0,
        }
    }
    fn CurrentChild(&self) -> Option<*const dyn Xplr>
    {
        match &self._Children {
            Some(buff) if self._Index < buff.Len() => {
                let node: &dyn Xplr = &*buff[self._Index];
                Some(node as *const dyn Xplr)
            }
            _ => None,
        }
    }
    fn Advance(&mut self) { self._Index += 1; }
}

//---------------------------------------------------------------------------------------------------------------------------------

/// Iterative depth-first traversal over an `Xplr` tree.
///
/// - `lambda(ancestors, enter)` is called with the ancestor path slice `&[&dyn Xplr]` and `enter` flag.
/// - If `lambda` returns `false` on entry (`enter == true`), the node is treated as a leaf-node
///   (its children are not traversed and no exit call is made).
/// - If `lambda` returns `false` on exit (`enter == false`), the entire rewind process is aborted immediately.
pub fn TraverseDepth<F>(root: &dyn Xplr, lambda: F)
    where F: FnMut(&[&dyn Xplr], bool) -> bool
{
    let roots = [root];
    TraverseDepthRoots(&roots, lambda);
}

/// Iterative depth-first traversal over multiple root `Xplr` nodes.
pub fn TraverseDepthRoots<F>(roots: &[&dyn Xplr], mut lambda: F)
    where F: FnMut(&[&dyn Xplr], bool) -> bool
{
    let mut tokStk: Vec<*const dyn Xplr> = Vec::with_capacity(roots.len().max(16));
    let mut childStk: Vec<ChildCursor> = Vec::with_capacity(roots.len().max(16));
    let mut ancStk: Vec<*const dyn Xplr> = Vec::with_capacity(32);
    let mut ancRefs: Vec<&dyn Xplr> = Vec::with_capacity(32);

    for &root in roots.iter().rev() {
        tokStk.push(root as *const dyn Xplr);
        childStk.push(ChildCursor::New());
    }

    while let (Some(&tok), Some(childIter)) = (tokStk.last(), childStk.last_mut()) {
        let mut enterFlg = false;

        if childIter.IsNull() {
            // begin child traversal
            ancStk.push(tok);
            ancRefs.clear();
            for &ptr in &ancStk {
                ancRefs.push(unsafe { &*ptr });
            }
            enterFlg = true;
            let res = lambda(&ancRefs, enterFlg); // play the tag and declare that children are coming
            if res {
                // if lambda returns false, we do not traverse children
                let node = unsafe { &*tok };
                if let Some(branch) = node.Branch() {
                    match branch.Children() {
                        Ok(children) => {
                            childIter._Children = Some(children);
                            childIter._Index = 0;
                        }
                        Err(_) => {
                            childIter._Children = Some(Buff::New());
                            childIter._Index = 0;
                        }
                    }
                } else {
                    childIter._Children = Some(Buff::New());
                    childIter._Index = 0;
                }
            } else {
                childIter._Children = Some(Buff::New());
                childIter._Index = 0;
            }
        }

        if childIter.Size() == 0 {
            // have no children left
            if !enterFlg {
                // No exit call, if it is a leaf
                ancRefs.clear();
                for &ptr in &ancStk {
                    ancRefs.push(unsafe { &*ptr });
                }
                let res = lambda(&ancRefs, false); // exit call for the tag
                if !res {
                    // if lambda returns false on exit the entire rewind process is aborted
                    return;
                }
            }
            tokStk.pop();
            childStk.pop();
            let tk = ancStk.pop();
            debug_assert_eq!(tk, Some(tok));
            continue;
        }

        let nxChild = childIter.CurrentChild()
                               .expect("Valid child must exist when Size > 0");
        childIter.Advance();

        tokStk.push(nxChild);
        childStk.push(ChildCursor::New());
    }
}
