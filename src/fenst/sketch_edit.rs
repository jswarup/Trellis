// sketch_edit.rs ---------------------------------------------------------------------------------------------------
//! Transactional atomic patch application, change sets, and bounded undo/redo history for `SketchDocument`.

use crate::fenst::sketch::{SketchBounds, SketchDocument, SketchEntity, SketchPoint};
use crate::fenst::sketch_dimension::SketchDimension;
use crate::silo::{Arr, Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------
// Atomic Mutation Patch (Non-Record)

/// Represents an immutable, atomic reversible modification to a sketch document.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchPatch
{
    _Added:              Buff<SketchEntity>,
    _Removed:            Buff<SketchEntity>,
    _Replaced:           Buff<(SketchEntity, SketchEntity)>,
    _Points:             Buff<SketchPoint>,
    _PointOffset:        u32,
    _AddedDimensions:    Buff<SketchDimension>,
    _RemovedDimensions:  Buff<SketchDimension>,
    _ReplacedDimensions: Buff<(SketchDimension, SketchDimension)>,
}

impl Default for SketchPatch
{
    fn default() -> Self { Self::Empty() }
}

impl SketchPatch
{
    pub fn Empty() -> Self
    {
        Self {
            _Added:              Buff::New(),
            _Removed:            Buff::New(),
            _Replaced:           Buff::New(),
            _Points:             Buff::New(),
            _PointOffset:        0,
            _AddedDimensions:    Buff::New(),
            _RemovedDimensions:  Buff::New(),
            _ReplacedDimensions: Buff::New(),
        }
    }

    pub fn New(added: Buff<SketchEntity>, removed: Buff<SketchEntity>,
               replaced: Buff<(SketchEntity, SketchEntity)>)
               -> Self
    {
        Self {
            _Added:              added,
            _Removed:            removed,
            _Replaced:           replaced,
            _Points:             Buff::New(),
            _PointOffset:        0,
            _AddedDimensions:    Buff::New(),
            _RemovedDimensions:  Buff::New(),
            _ReplacedDimensions: Buff::New(),
        }
    }

    pub fn DimensionAdd(dim: SketchDimension) -> Self
    {
        let mut added = Stash::WithCapacity(1);
        added.Push(dim);
        Self {
            _AddedDimensions: added.IntoBuff(),
            ..Self::Empty()
        }
    }

    pub fn DimensionRemove(dim: SketchDimension) -> Self
    {
        let mut removed = Stash::WithCapacity(1);
        removed.Push(dim);
        Self {
            _RemovedDimensions: removed.IntoBuff(),
            ..Self::Empty()
        }
    }

    pub fn Added(&self) -> Arr<'_, SketchEntity> { self._Added.Arr() }

    pub fn Removed(&self) -> Arr<'_, SketchEntity> { self._Removed.Arr() }

    pub fn Replaced(&self) -> Arr<'_, (SketchEntity, SketchEntity)> { self._Replaced.Arr() }

    pub fn AddedCount(&self) -> u32 { self._Added.Size() }

    pub fn RemovedCount(&self) -> u32 { self._Removed.Size() }

    pub fn ReplacedCount(&self) -> u32 { self._Replaced.Size() }

    pub fn AddedDimensions(&self) -> Arr<'_, SketchDimension> { self._AddedDimensions.Arr() }

    pub fn RemovedDimensions(&self) -> Arr<'_, SketchDimension> { self._RemovedDimensions.Arr() }

    pub fn IsEmpty(&self) -> bool
    {
        self._Added.IsEmpty()
            && self._Removed.IsEmpty()
            && self._Replaced.IsEmpty()
            && self._AddedDimensions.IsEmpty()
            && self._RemovedDimensions.IsEmpty()
            && self._ReplacedDimensions.IsEmpty()
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

        let mut inv_add_dims = Stash::WithCapacity(self._RemovedDimensions.Size());
        let mut inv_rem_dims = Stash::WithCapacity(self._AddedDimensions.Size());
        let mut inv_rep_dims = Stash::WithCapacity(self._ReplacedDimensions.Size());

        self._RemovedDimensions.Arr().Traverse(|d| inv_add_dims.Push(*d));
        self._AddedDimensions.Arr().Traverse(|d| inv_rem_dims.Push(*d));
        self._ReplacedDimensions.Arr().Traverse(|(old, new)| inv_rep_dims.Push((*new, *old)));

        Self {
            _Added:              inv_added.IntoBuff(),
            _Removed:            inv_removed.IntoBuff(),
            _Replaced:           inv_replaced.IntoBuff(),
            _Points:             self._Points.clone(),
            _PointOffset:        self._PointOffset,
            _AddedDimensions:    inv_add_dims.IntoBuff(),
            _RemovedDimensions:  inv_rem_dims.IntoBuff(),
            _ReplacedDimensions: inv_rep_dims.IntoBuff(),
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Builder for Assembling a Patch

#[derive(Default)]
pub struct SketchPatchBuilder
{
    _Added:               Stash<SketchEntity>,
    _Removed:             Stash<SketchEntity>,
    _Replaced:            Stash<(SketchEntity, SketchEntity)>,
    _Points:              Buff<SketchPoint>,
    _PointOffset:         u32,
    _AddedDimensions:     Stash<SketchDimension>,
    _RemovedDimensions:   Stash<SketchDimension>,
    _ReplacedDimensions:  Stash<(SketchDimension, SketchDimension)>,
}

impl SketchPatchBuilder
{
    pub fn New() -> Self
    {
        Self {
            _Added:               Stash::New(),
            _Removed:             Stash::New(),
            _Replaced:            Stash::New(),
            _Points:              Buff::New(),
            _PointOffset:         0,
            _AddedDimensions:     Stash::New(),
            _RemovedDimensions:   Stash::New(),
            _ReplacedDimensions:  Stash::New(),
        }
    }

    pub fn AddEntity(&mut self, entity: SketchEntity) { self._Added.Push(entity); }

    pub fn RemoveEntity(&mut self, entity: SketchEntity) { self._Removed.Push(entity); }

    pub fn AddDimension(&mut self, dim: SketchDimension) { self._AddedDimensions.Push(dim); }

    pub fn RemoveDimension(&mut self, dim: SketchDimension) { self._RemovedDimensions.Push(dim); }

    pub fn ReplaceDimension(&mut self, old: SketchDimension, new: SketchDimension)
    {
        self._ReplacedDimensions.Push((old, new));
    }

    pub fn AppendPoints(&mut self, offset: u32, points: Arr<'_, SketchPoint>)
    {
        self._PointOffset = offset;
        self._Points = Buff::FromDispenser(points.Size(), |i| points[i]);
    }

    pub fn ReplaceEntity(&mut self, old: SketchEntity, new: SketchEntity)
    {
        self._Replaced.Push((old, new));
    }

    pub fn Build(mut self) -> SketchPatch
    {
        SketchPatch {
            _Added:              self._Added.TakeBuff(),
            _Removed:            self._Removed.TakeBuff(),
            _Replaced:           self._Replaced.TakeBuff(),
            _Points:             self._Points,
            _PointOffset:        self._PointOffset,
            _AddedDimensions:    self._AddedDimensions.TakeBuff(),
            _RemovedDimensions:  self._RemovedDimensions.TakeBuff(),
            _ReplacedDimensions: self._ReplacedDimensions.TakeBuff(),
        }
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
    _UndoStack:        Stash<SketchPatch>,
    _RedoStack:        Stash<SketchPatch>,
    _SavedRevision:    u64,
    _MaxDepth:         u32,
    _UndoStates:       Stash<(u64, u64)>,
    _RedoStates:       Stash<(u64, u64)>,
    _State:            u64,
    _SavedState:       u64,
    _NextState:        u64,
    _ObservedRevision: u64,
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
               _MaxDepth:      max_depth.max(1), _UndoStates: Stash::New(), _RedoStates: Stash::New(), _State: 0, _SavedState: 0, _NextState: 1, _ObservedRevision: 0, }
    }

    pub fn CanUndo(&self) -> bool { !self._UndoStack.IsEmpty() }

    pub fn CanRedo(&self) -> bool { !self._RedoStack.IsEmpty() }

    pub fn UndoCount(&self) -> u32 { self._UndoStack.Size() }

    pub fn RedoCount(&self) -> u32 { self._RedoStack.Size() }

    pub fn MarkSaved(&mut self, current_revision: u64)
    {
        self._SavedRevision = current_revision;
        self._SavedState = self._State;
        self._ObservedRevision = current_revision;
    }

    pub fn IsDirty(&self, current_revision: u64) -> bool
    {
        self._SavedState != self._State || self._ObservedRevision != current_revision
    }

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
        self._RedoStates.Clear();

        // Enforce max depth
        if self._UndoStack.Size() >= self._MaxDepth {
            let mut pruned = Stash::WithCapacity(self._MaxDepth);
            let mut states = Stash::WithCapacity(self._MaxDepth);
            let arr = self._UndoStack.Arr();
            let mut i = 1u32;
            while i < arr.Len() {
                pruned.Push(arr.Get(i).unwrap().clone());
                states.Push(self._UndoStates[i]);
                i += 1;
            }
            self._UndoStack = pruned;
            self._UndoStates = states;
        }

        self._UndoStack.Push(patch);
        self._UndoStates.Push((self._State, self._NextState));
        self._State = self._NextState;
        self._NextState += 1;
        self._ObservedRevision = doc.Revision();
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
        let states = self._UndoStates.Pop().unwrap();
        self._State = states.0;
        self._RedoStates.Push(states);
        self._ObservedRevision = doc.Revision();
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
        let states = self._RedoStates.Pop().unwrap();
        self._State = states.1;
        self._UndoStates.Push(states);
        self._ObservedRevision = doc.Revision();
        Ok(Some(change_set))
    }

    fn ApplyPatchInternal(doc: &mut SketchDocument, patch: &SketchPatch)
                          -> Result<SketchChangeSet, String>
    {
        if !patch._Points.IsEmpty() {
            doc.AppendPointData(patch._PointOffset, patch._Points.Arr())?;
        }
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
            // F04: Remove linked annotations if target entity was deleted
            let mut del_dims = Stash::New();
            doc.Dimensions().Traverse(|dim| {
                if dim._TargetEntityId == rem.Id() {
                    del_dims.Push(dim._Id);
                }
            });
            del_dims.Arr().Traverse(|id| {
                doc.RemoveDimension(*id);
            });
        });

        patch._Replaced.Arr().Traverse(|(old_ent, new_ent)| {
            affected.Push(old_ent.Id());
            if let Some(existing) = doc.FindEntityMut(old_ent.Id()) {
                *existing = new_ent.clone();
            }
            // F04: Keep linked dimensions consistent with modified entity
            let mut updated_dims = Stash::New();
            doc.Dimensions().Traverse(|dim| {
                if dim._TargetEntityId == new_ent.Id() {
                    let mut updated = *dim;
                    match (dim._Kind, new_ent.Primitive()) {
                        (crate::fenst::sketch_dimension::DimensionKind::LinearAligned
                         | crate::fenst::sketch_dimension::DimensionKind::LinearHorizontal
                         | crate::fenst::sketch_dimension::DimensionKind::LinearVertical,
                         crate::fenst::sketch::SketchPrimitive::Line(l)) => {
                            updated._P1 = l._Start;
                            updated._P2 = l._End;
                            updated_dims.Push(updated);
                        }
                        (crate::fenst::sketch_dimension::DimensionKind::Radial
                         | crate::fenst::sketch_dimension::DimensionKind::Diametral,
                         crate::fenst::sketch::SketchPrimitive::Circle(c)) => {
                            updated._P1 = c._Center;
                            updated._P2 = SketchPoint::New(c._Center._X + c._Radius, c._Center._Y);
                            updated_dims.Push(updated);
                        }
                        (crate::fenst::sketch_dimension::DimensionKind::Radial
                         | crate::fenst::sketch_dimension::DimensionKind::Diametral,
                         crate::fenst::sketch::SketchPrimitive::Arc(a)) => {
                            updated._P1 = a._Center;
                            updated._P2 = SketchPoint::New(
                                a._Center._X + a._Radius * a._StartAngle.cos(),
                                a._Center._Y + a._Radius * a._StartAngle.sin(),
                            );
                            updated_dims.Push(updated);
                        }
                        _ => {}
                    }
                }
            });
            updated_dims.Arr().Traverse(|d| {
                doc.InsertDimension(*d);
            });
        });

        patch._Added.Arr().Traverse(|add| {
            affected.Push(add.Id());
            let _ = doc.RemoveEntity(add.Id()); // in case re-inserting during redo
            doc.InsertEntity(add.clone());
        });

        // Apply dimension mutations
        patch._RemovedDimensions.Arr().Traverse(|rem| {
            let _ = doc.RemoveDimension(rem._Id);
        });

        patch._ReplacedDimensions.Arr().Traverse(|(_old, new)| {
            doc.InsertDimension(*new);
        });

        patch._AddedDimensions.Arr().Traverse(|add| {
            doc.InsertDimension(*add);
        });

        if !patch._Replaced.IsEmpty()
            || !patch._AddedDimensions.IsEmpty()
            || !patch._RemovedDimensions.IsEmpty()
            || !patch._ReplacedDimensions.IsEmpty()
        {
            doc.Touch();
        }

        Ok(SketchChangeSet::New(affected.IntoBuff(),
                                dirty_bounds,
                                doc.Revision()))
    }
}

//-------------------------------------------------------------------------------------------------
