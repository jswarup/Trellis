//-- sketch_xplr.rs -------------------------------------------------------------------------------------------------
use crate::fenst::sketch::{
    Circle, CircularArc, Ellipse, LineSegment, PolylineHeader, SketchDocument,
    SketchEntity, SketchGroup, SketchLayer, SketchPrimitive,
};
use crate::fenst::xplr::{BranchXplr, LeafXplr, StreamChunk, Xplr};
use crate::silo::{Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------

/// Root explorer node projecting a `SketchDocument` into an `Xplr` tree hierarchy.
pub struct SketchXplr
{
    _Name:     String,
    _Path:     String,
    _Layers:   Buff<SketchLayer>,
    _Groups:   Buff<SketchGroup>,
    _Entities: Buff<SketchEntity>,
    _Details:  Buff<String>,
}

impl SketchXplr
{
    pub fn FromDocument(doc: &SketchDocument, name: &str, path: &str) -> Self
    {
        let mut layersStash = Stash::New();
        doc.Layers().Traverse(|layer| {
            layersStash.Push(layer.clone());
        });

        let mut groupsStash = Stash::New();
        doc.Groups().Traverse(|group| {
            groupsStash.Push(group.clone());
        });

        let mut entitiesStash = Stash::New();
        let mut detailsStash = Stash::New();
        doc.Entities().Traverse(|entity| {
            entitiesStash.Push(entity.clone());
            let detail = match entity.Primitive() {
                SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                    let len = (_End._X - _Start._X).hypot(_End._Y - _Start._Y);
                    format!("Line {} [L={:.2}mm]", entity.Id(), len)
                }
                SketchPrimitive::Circle(Circle { _Radius, .. }) => {
                    format!("Circle {} [R={:.2}mm]", entity.Id(), _Radius)
                }
                SketchPrimitive::Arc(CircularArc {
                    _Radius,
                    _SweepAngle,
                    ..
                }) => {
                    let deg = _SweepAngle.to_degrees().abs();
                    format!("Arc {} [R={:.2}mm, Sweep={:.1}°]", entity.Id(), _Radius, deg)
                }
                SketchPrimitive::Ellipse(Ellipse {
                    _SemiMajor,
                    _SemiMinor,
                    ..
                }) => {
                    format!(
                        "Ellipse {} [A={:.2}mm, B={:.2}mm]",
                        entity.Id(),
                        _SemiMajor,
                        _SemiMinor
                    )
                }
                SketchPrimitive::Polyline(PolylineHeader {
                    _PointCount,
                    _IsClosed,
                    ..
                }) => {
                    let closedStr = if *_IsClosed { "Closed" } else { "Open" };
                    format!("Polyline {} [{} pts, {}]", entity.Id(), _PointCount, closedStr)
                }
            };
            detailsStash.Push(detail);
        });

        Self {
            _Name:     name.to_string(),
            _Path:     path.to_string(),
            _Layers:   layersStash.IntoBuff(),
            _Groups:   groupsStash.IntoBuff(),
            _Entities: entitiesStash.IntoBuff(),
            _Details:  detailsStash.IntoBuff(),
        }
    }

    pub fn LayerCount(&self) -> u32
    {
        self._Layers.Len()
    }

    pub fn GroupCount(&self) -> u32
    {
        self._Groups.Len()
    }

    pub fn EntityCount(&self) -> u32
    {
        self._Entities.Len()
    }
}

impl Xplr for SketchXplr
{
    fn Name(&self) -> &str
    {
        &self._Name
    }

    fn Path(&self) -> &str
    {
        &self._Path
    }

    fn IsLeaf(&self) -> bool
    {
        false
    }

    fn Branch(&self) -> Option<&dyn BranchXplr>
    {
        Some(self)
    }

    fn AsAny(&self) -> Option<&dyn std::any::Any>
    {
        Some(self)
    }
}

impl BranchXplr for SketchXplr
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut children = Stash::New();

        let layersBranch = SketchLayersBranch::New(
            format!("{}/layers", self._Path),
            self._Layers.clone(),
        );
        children.Push(Box::new(layersBranch) as Box<dyn Xplr>);

        let groupsBranch = SketchGroupsBranch::New(
            format!("{}/groups", self._Path),
            self._Groups.clone(),
        );
        children.Push(Box::new(groupsBranch) as Box<dyn Xplr>);

        let entitiesBranch = SketchEntitiesBranch::New(
            format!("{}/entities", self._Path),
            self._Entities.clone(),
            self._Details.clone(),
        );
        children.Push(Box::new(entitiesBranch) as Box<dyn Xplr>);

        Ok(children.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(3)
    }
}

//-------------------------------------------------------------------------------------------------

/// Branch node representing the collection of sketch layers.
pub struct SketchLayersBranch
{
    _Path:   String,
    _Layers: Buff<SketchLayer>,
}

impl SketchLayersBranch
{
    pub fn New(path: String, layers: Buff<SketchLayer>) -> Self
    {
        Self {
            _Path:   path,
            _Layers: layers,
        }
    }
}

impl Xplr for SketchLayersBranch
{
    fn Name(&self) -> &str
    {
        "Layers"
    }

    fn Path(&self) -> &str
    {
        &self._Path
    }

    fn IsLeaf(&self) -> bool
    {
        false
    }

    fn Branch(&self) -> Option<&dyn BranchXplr>
    {
        Some(self)
    }
}

impl BranchXplr for SketchLayersBranch
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut children = Stash::New();
        let parentPath = self._Path.clone();
        self._Layers.Traverse(|layer| {
            let leaf = SketchLayerLeaf::New(
                format!("{}/{}", parentPath, layer.Id()),
                layer.clone(),
            );
            children.Push(Box::new(leaf) as Box<dyn Xplr>);
        });
        Ok(children.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(self._Layers.Len())
    }
}

//-------------------------------------------------------------------------------------------------

/// Leaf node for an individual sketch layer.
pub struct SketchLayerLeaf
{
    _Path:  String,
    _Layer: SketchLayer,
    _Title: String,
}

impl SketchLayerLeaf
{
    pub fn New(path: String, layer: SketchLayer) -> Self
    {
        let title = format!(
            "Layer {}: \"{}\" [{}{}]",
            layer.Id(),
            layer.Name(),
            if layer.IsVisible() { "visible" } else { "hidden" },
            if layer.IsLocked() { ", locked" } else { "" }
        );
        Self {
            _Path:  path,
            _Layer: layer,
            _Title: title,
        }
    }

    pub fn Layer(&self) -> &SketchLayer
    {
        &self._Layer
    }
}

impl Xplr for SketchLayerLeaf
{
    fn Name(&self) -> &str
    {
        &self._Title
    }

    fn Path(&self) -> &str
    {
        &self._Path
    }

    fn IsLeaf(&self) -> bool
    {
        true
    }

    fn Leaf(&self) -> Option<&dyn LeafXplr>
    {
        Some(self)
    }

    fn AsAny(&self) -> Option<&dyn std::any::Any>
    {
        Some(self)
    }
}

impl LeafXplr for SketchLayerLeaf
{
    fn Size(&self) -> u64
    {
        self._Title.len() as u64
    }

    fn Extension(&self) -> &str
    {
        "layer"
    }

    fn ReadChunk(&self, offset: u64, length: u32) -> Result<StreamChunk, String>
    {
        let total = self._Title.len() as u64;
        let start = offset as usize;
        let end = (start + length as usize).min(self._Title.len());
        let slice = if start < self._Title.len() {
            &self._Title[start..end]
        } else {
            ""
        };
        Ok(StreamChunk::New(
            self._Path.clone(),
            offset,
            total,
            slice.to_string(),
        ))
    }
}

//-------------------------------------------------------------------------------------------------

/// Branch node representing the collection of sketch groups.
pub struct SketchGroupsBranch
{
    _Path:   String,
    _Groups: Buff<SketchGroup>,
}

impl SketchGroupsBranch
{
    pub fn New(path: String, groups: Buff<SketchGroup>) -> Self
    {
        Self {
            _Path:   path,
            _Groups: groups,
        }
    }
}

impl Xplr for SketchGroupsBranch
{
    fn Name(&self) -> &str
    {
        "Groups"
    }

    fn Path(&self) -> &str
    {
        &self._Path
    }

    fn IsLeaf(&self) -> bool
    {
        false
    }

    fn Branch(&self) -> Option<&dyn BranchXplr>
    {
        Some(self)
    }
}

impl BranchXplr for SketchGroupsBranch
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut children = Stash::New();
        let parentPath = self._Path.clone();
        self._Groups.Traverse(|group| {
            let leaf = SketchGroupLeaf::New(
                format!("{}/{}", parentPath, group.Id()),
                group.clone(),
            );
            children.Push(Box::new(leaf) as Box<dyn Xplr>);
        });
        Ok(children.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(self._Groups.Len())
    }
}

//-------------------------------------------------------------------------------------------------

/// Leaf node representing an individual sketch group.
pub struct SketchGroupLeaf
{
    _Path:  String,
    _Group: SketchGroup,
    _Title: String,
}

impl SketchGroupLeaf
{
    pub fn New(path: String, group: SketchGroup) -> Self
    {
        let title = format!(
            "Group {} [\"{}\", {} members]",
            group.Id(),
            group.Name(),
            group.MemberIds().Len()
        );
        Self {
            _Path:  path,
            _Group: group,
            _Title: title,
        }
    }

    pub fn Group(&self) -> &SketchGroup
    {
        &self._Group
    }
}

impl Xplr for SketchGroupLeaf
{
    fn Name(&self) -> &str
    {
        &self._Title
    }

    fn Path(&self) -> &str
    {
        &self._Path
    }

    fn IsLeaf(&self) -> bool
    {
        true
    }

    fn Leaf(&self) -> Option<&dyn LeafXplr>
    {
        Some(self)
    }

    fn AsAny(&self) -> Option<&dyn std::any::Any>
    {
        Some(self)
    }
}

impl LeafXplr for SketchGroupLeaf
{
    fn Size(&self) -> u64
    {
        self._Title.len() as u64
    }

    fn Extension(&self) -> &str
    {
        "group"
    }

    fn ReadChunk(&self, offset: u64, length: u32) -> Result<StreamChunk, String>
    {
        let total = self._Title.len() as u64;
        let start = offset as usize;
        let end = (start + length as usize).min(self._Title.len());
        let slice = if start < self._Title.len() {
            &self._Title[start..end]
        } else {
            ""
        };
        Ok(StreamChunk::New(
            self._Path.clone(),
            offset,
            total,
            slice.to_string(),
        ))
    }
}

//-------------------------------------------------------------------------------------------------

/// Branch node representing the collection of sketch entities.
pub struct SketchEntitiesBranch
{
    _Path:     String,
    _Entities: Buff<SketchEntity>,
    _Details:  Buff<String>,
}

impl SketchEntitiesBranch
{
    pub fn New(
        path: String,
        entities: Buff<SketchEntity>,
        details: Buff<String>,
    ) -> Self
    {
        Self {
            _Path:     path,
            _Entities: entities,
            _Details:  details,
        }
    }
}

impl Xplr for SketchEntitiesBranch
{
    fn Name(&self) -> &str
    {
        "Entities"
    }

    fn Path(&self) -> &str
    {
        &self._Path
    }

    fn IsLeaf(&self) -> bool
    {
        false
    }

    fn Branch(&self) -> Option<&dyn BranchXplr>
    {
        Some(self)
    }
}

impl BranchXplr for SketchEntitiesBranch
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut children = Stash::New();
        let parentPath = self._Path.clone();
        let details = self._Details.Arr();
        let mut idx = 0u32;
        self._Entities.Traverse(|entity| {
            let detail = details.Get(idx).cloned().unwrap_or_default();
            let leaf = SketchEntityLeaf::New(
                format!("{}/{}", parentPath, entity.Id()),
                entity.clone(),
                detail,
            );
            children.Push(Box::new(leaf) as Box<dyn Xplr>);
            idx += 1;
        });
        Ok(children.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(self._Entities.Len())
    }
}

//-------------------------------------------------------------------------------------------------

/// Leaf node representing an individual sketch entity.
pub struct SketchEntityLeaf
{
    _Path:     String,
    _Entity:   SketchEntity,
    _Detail:   String,
}

impl SketchEntityLeaf
{
    pub fn New(path: String, entity: SketchEntity, detail: String) -> Self
    {
        Self {
            _Path:     path,
            _Entity:   entity,
            _Detail:   detail,
        }
    }

    pub fn Entity(&self) -> &SketchEntity
    {
        &self._Entity
    }

    pub fn EntityId(&self) -> u32
    {
        self._Entity.Id()
    }
}

impl Xplr for SketchEntityLeaf
{
    fn Name(&self) -> &str
    {
        &self._Detail
    }

    fn Path(&self) -> &str
    {
        &self._Path
    }

    fn IsLeaf(&self) -> bool
    {
        true
    }

    fn Leaf(&self) -> Option<&dyn LeafXplr>
    {
        Some(self)
    }

    fn AsAny(&self) -> Option<&dyn std::any::Any>
    {
        Some(self)
    }
}

impl LeafXplr for SketchEntityLeaf
{
    fn Size(&self) -> u64
    {
        self._Detail.len() as u64
    }

    fn Extension(&self) -> &str
    {
        "entity"
    }

    fn ReadChunk(&self, offset: u64, length: u32) -> Result<StreamChunk, String>
    {
        let total = self._Detail.len() as u64;
        let start = offset as usize;
        let end = (start + length as usize).min(self._Detail.len());
        let slice = if start < self._Detail.len() {
            &self._Detail[start..end]
        } else {
            ""
        };
        Ok(StreamChunk::New(
            self._Path.clone(),
            offset,
            total,
            slice.to_string(),
        ))
    }
}

//-------------------------------------------------------------------------------------------------
