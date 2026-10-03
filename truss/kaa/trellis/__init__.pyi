# __init__.pyi -----------------------------------------------------------------------------------

from typing import List, Tuple

__version__: str

def version() -> str:
    """Returns the version of the Trellis framework."""
    ...

def load_geometry(path: str) -> GeometryAsset:
    """Convenience helper to load a 3D geometry file (.obj or .pts)."""
    ...

class GeometryAsset:
    """Validated, normalized 3D geometry asset (polygonal mesh or point cloud)."""

    @property
    def vertex_count(self) -> int:
        """Total number of vertices."""
        ...

    @property
    def face_count(self) -> int:
        """Total number of polygonal/triangular faces."""
        ...

    @property
    def point_count(self) -> int:
        """Total number of points in the cloud or mesh."""
        ...

    @property
    def is_point_cloud(self) -> bool:
        """True if asset represents a point cloud, false if a polygonal mesh."""
        ...

    @property
    def bounds(self) -> Tuple[Tuple[float, float, float], Tuple[float, float, float]]:
        """Source axis-aligned bounding box: ((min_x, min_y, min_z), (max_x, max_y, max_z))."""
        ...

    def vertex_positions(self) -> List[Tuple[float, float, float]]:
        """Returns normalized 3D vertex positions in local space [-1.0, 1.0]."""
        ...

    def vertex_colors(self) -> List[Tuple[float, float, float, float]]:
        """Returns RGBA vertex colors in range [0.0, 1.0]."""
        ...

    def faces(self) -> List[Tuple[int, int, int]]:
        """Returns triangular face vertex index tuples (i0, i1, i2)."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class GeometryService:
    """Service for loading and parsing 3D geometry assets."""

    def __init__(self) -> None: ...

    def load(self, path: str) -> GeometryAsset:
        """Loads a .obj or .pts file from disk."""
        ...

    def parse_obj(self, data: str) -> GeometryAsset:
        """Parses a Wavefront OBJ string."""
        ...

    def parse_pts(self, data: str) -> GeometryAsset:
        """Parses a PTS point cloud string."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Session:
    """Top-level session providing access to Trellis services and workflows."""

    def __init__(self) -> None: ...

    @property
    def geometry(self) -> GeometryService:
        """Access the geometry service."""
        ...

    def load_geometry(self, path: str) -> GeometryAsset:
        """Convenience shortcut to load a 3D geometry file (.obj, .pts)."""
        ...

    def parse_obj(self, data: str) -> GeometryAsset:
        """Convenience shortcut to parse Wavefront OBJ string."""
        ...

    def parse_pts(self, data: str) -> GeometryAsset:
        """Convenience shortcut to parse PTS point cloud string."""
        ...

    def version(self) -> str:
        """Returns the Trellis framework version."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
