// sketch_edit.rs ---------------------------------------------------------------------------------------------------
//! Transactional atomic patch application, change sets, and bounded undo/redo history for `SketchDocument`.

use crate::fenst::sketch::{SketchBounds, SketchDocument, SketchEntity};
use crate::silo::{Arr, Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------
// Atomic Mutation Patch (Non-Record)

/// Represents an immutable, atomic reversible modification to a sketch document.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchPatch
{
    _Added:    Buff<SketchEntity>,
    _Removed:  Buff<SketchEntity>,
    _Replaced: Buff<(SketchEntity, SketchEntity)>,
}

impl Default for SketchPatch
{
    fn default() -> Self { Self::Empty() }
}

impl SketchPatch
{
    pub fn Empty() -> Self
    {
        Self { _Added:    Buff::New(),
               _Removed:  Buff::New(),
               _Replaced: Buff::New(), }
    }

    pub fn New(added: Buff<SketchEntity>, removed: Buff<SketchEntity>,
               replaced: Buff<(SketchEntity, SketchEntity)>)
               -> Self
    {
        Self { _Added:    added,
               _Removed:  removed,
               _Replaced: replaced, }
    }

    pub fn Added(&self) -> Arr<'_, SketchEntity> { self._Added.Arr() }

    pub fn Removed(&self) -> Arr<'_, SketchEntity> { self._Removed.Arr() }

    pub fn Replaced(&self) -> Arr<'_, (SketchEntity, SketchEntity)> { self._Replaced.Arr() }

    pub fn AddedCount(&self) -> u32 { self._Added.Size() }

    pub fn RemovedCount(&self) -> u32 { self._Removed.Size() }

    pub fn ReplacedCount(&self) -> u32 { self._Replaced.Size() }

    pub fn IsEmpty(&self) -> bool
    {
        self._Added.IsEmpty() && self._Removed.IsEmpty() && self._Replaced.IsEmpty()
    }

    pub fn Invert(&self) -> Self
    {
        let mut inv_added = Stash::WithCapacity(self._Removed.Size());
        let mut inv_removed = Stash::WithCapacity(self._Added.Size());
        let mut inv_replaced = Stash::WithCapacity(self._Replaced.Size());

        self._Removed.Arr().Traverse(|e| {
                               inv_added.Push(e.clone());
                           });

        self._Added.Arr().Traverse(|e| {
                             inv_removed.Push(e.clone());
                         });

        self._Replaced.Arr().Traverse(|(old, new)| {
                                inv_replaced.Push((new.clone(), old.clone()));
                            });

        Self { _Added:    inv_added.IntoBuff(),
               _Removed:  inv_removed.IntoBuff(),
               _Replaced: inv_replaced.IntoBuff(), }
    }
}

//-------------------------------------------------------------------------------------------------
// Builder for Assembling a Patch

#[derive(Default)]
pub struct SketchPatchBuilder
{
    _Added:    Stash<SketchEntity>,
    _Removed:  Stash<SketchEntity>,
    _Replaced: Stash<(SketchEntity, SketchEntity)>,
}

impl SketchPatchBuilder
{
    pub fn New() -> Self
    {
        Self { _Added:    Stash::New(),
               _Removed:  Stash::New(),
               _Replaced: Stash::New(), }
    }

    pub fn AddEntity(&mut self, entity: SketchEntity) { self._Added.Push(entity); }

    pub fn RemoveEntity(&mut self, entity: SketchEntity) { self._Removed.Push(entity); }

    pub fn ReplaceEntity(&mut self, old: SketchEntity, new: SketchEntity)
    {
        self._Replaced.Push((old, new));
    }

    pub fn Build(mut self) -> SketchPatch
    {
        SketchPatch { _Added:    self._Added.TakeBuff(),
                      _Removed:  self._Removed.TakeBuff(),
                      _Replaced: self._Replaced.TakeBuff(), }
    }
}

//-------------------------------------------------------------------------------------------------
// Mutation Notification ChangeSet Record (DTO)

/// Diagnostic delta emitted when a patch is committed, allowing targeted cache invalidation.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchChangeSet
{
    pub _AffectedIds: Buff<u32>,
    pub _DirtyBounds: SketchBounds,
    pub _Revision:    u64,
}

impl SketchChangeSet
{
    pub fn New(affected_ids: Buff<u32>, dirty_bounds: SketchBounds, revision: u64) -> Self
    {
        Self { _AffectedIds: affected_ids,
               _DirtyBounds: dirty_bounds,
               _Revision:    revision, }
    }
}

//-------------------------------------------------------------------------------------------------
// Bounded Undo / Redo History Engine (Non-Record)

/// Bounded undo/redo history manager tracking document revisions and dirty status.
pub struct SketchHistory
{
    _UndoStack:     Stash<SketchPatch>,
    _RedoStack:     Stash<SketchPatch>,
    _SavedRevision: u64,
    _MaxDepth:      u32,
}

impl Default for SketchHistory
{
    fn default() -> Self { Self::New(128) }
}

impl SketchHistory
{
    pub fn New(max_depth: u32) -> Self
    {
        Self { _UndoStack:     Stash::New(),
               _RedoStack:     Stash::New(),
               _SavedRevision: 0,
               _MaxDepth:      max_depth.max(1), }
    }

    pub fn CanUndo(&self) -> bool { !self._UndoStack.IsEmpty() }

    pub fn CanRedo(&self) -> bool { !self._RedoStack.IsEmpty() }

    pub fn UndoCount(&self) -> u32 { self._UndoStack.Size() }

    pub fn RedoCount(&self) -> u32 { self._RedoStack.Size() }

    pub fn MarkSaved(&mut self, current_revision: u64) { self._SavedRevision = current_revision; }

    pub fn IsDirty(&self, current_revision: u64) -> bool { self._SavedRevision != current_revision }

    /// Applies a validated forward patch to the document and records it on the undo stack.
    pub fn CommitPatch(&mut self, doc: &mut SketchDocument, patch: SketchPatch)
                       -> Result<SketchChangeSet, String>
    {
        if patch.IsEmpty() {
            return Ok(SketchChangeSet::New(Buff::New(),
                                           SketchBounds::Empty(),
                                           doc.Revision()));
        }

        let change_set = Self::ApplyPatchInternal(doc, &patch)?;

        // Pushing a new commit invalidates the redo stack
        self._RedoStack.Clear();

        // Enforce max depth
        if self._UndoStack.Size() >= self._MaxDepth {
            let mut pruned = Stash::WithCapacity(self._MaxDepth);
            let arr = self._UndoStack.Arr();
            let mut i = 1u32;
            while i < arr.Len() {
                pruned.Push(arr.Get(i).unwrap().clone());
                i += 1;
            }
            self._UndoStack = pruned;
        }

        self._UndoStack.Push(patch);
        Ok(change_set)
    }

    /// Undoes the most recent patch and pushes its inverse onto the redo stack.
    pub fn Undo(&mut self, doc: &mut SketchDocument) -> Result<Option<SketchChangeSet>, String>
    {
        let Some(patch) = self._UndoStack.Pop() else {
            return Ok(None);
        };

        let inv_patch = patch.Invert();
        let change_set = Self::ApplyPatchInternal(doc, &inv_patch)?;
        self._RedoStack.Push(patch);
        Ok(Some(change_set))
    }

    /// Redoes the most recently undone patch.
    pub fn Redo(&mut self, doc: &mut SketchDocument) -> Result<Option<SketchChangeSet>, String>
    {
        let Some(patch) = self._RedoStack.Pop() else {
            return Ok(None);
        };

        let change_set = Self::ApplyPatchInternal(doc, &patch)?;
        self._UndoStack.Push(patch);
        Ok(Some(change_set))
    }

    fn ApplyPatchInternal(doc: &mut SketchDocument, patch: &SketchPatch)
                          -> Result<SketchChangeSet, String>
    {
        let mut affected = Stash::New();
        let mut dirty_bounds = SketchBounds::Empty();

        // Pass 1: Compute dirty bounds while doc is borrowed immutably
        {
            let arena = doc.PointArena();
            patch._Removed.Arr().Traverse(|rem| {
                                    dirty_bounds.ExpandByBounds(rem.Bounds(arena));
                                });
            patch._Replaced.Arr().Traverse(|(old_ent, new_ent)| {
                                     dirty_bounds.ExpandByBounds(old_ent.Bounds(arena));
                                     dirty_bounds.ExpandByBounds(new_ent.Bounds(arena));
                                 });
            patch._Added.Arr().Traverse(|add| {
                                  dirty_bounds.ExpandByBounds(add.Bounds(arena));
                              });
        }

        // Pass 2: Apply mutations to doc
        patch._Removed.Arr().Traverse(|rem| {
                                affected.Push(rem.Id());
                                let _ = doc.RemoveEntity(rem.Id());
                            });

        patch._Replaced.Arr().Traverse(|(old_ent, new_ent)| {
                                 affected.Push(old_ent.Id());
                                 if let Some(existing) = doc.FindEntityMut(old_ent.Id()) {
                                     *existing = new_ent.clone();
                                 }
                             });

        patch._Added.Arr().Traverse(|add| {
                              affected.Push(add.Id());
                              let _ = doc.RemoveEntity(add.Id()); // in case re-inserting during redo
                              doc.InsertEntity(add.clone());
                          });

        Ok(SketchChangeSet::New(affected.IntoBuff(),
                                dirty_bounds,
                                doc.Revision()))
    }
}

//-------------------------------------------------------------------------------------------------
