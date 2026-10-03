# Trellis Python Binding

Python interface for the **Trellis** algorithms and systems framework.

## Installation

### From Source (Developer Install)

Using `maturin` (recommended):

```bash
# In the Trellis root directory:
pip install maturin
maturin develop --features extension-module
```

Or using `pip`:

```bash
pip install ./truss/kaa
```

### Prebuilt Wheels

When wheels are built via `maturin build --release --features extension-module`:

```bash
pip install target/wheels/trellis-*.whl
```

## Quick Start

```python
import trellis

# 1. Initialize a session
session = trellis.Session()
print(f"Trellis Version: {session.version()}")

# 2. Parse 3D geometry from Wavefront OBJ
obj_data = """
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
"""
asset = session.parse_obj(obj_data)
print(f"Vertices: {asset.vertex_count}")
print(f"Faces:    {asset.face_count}")
print(f"Positions: {asset.vertex_positions()}")
print(f"Triangles: {asset.faces()}")

# 3. Parse PTS point cloud data
pts_data = """2
1.0 2.0 3.0 200 255 0 0
4.0 5.0 6.0 100 0 255 0
"""
cloud = session.parse_pts(pts_data)
print(f"Point Cloud Points: {cloud.point_count}")
print(f"Colors: {cloud.vertex_colors()}")

# 4. Load from file
# mesh = session.load_geometry("models/teapot.obj")
```

## Jupyter Notebook Support

The package installs standard type stubs and metadata, enabling autocompletion and interactive inspection in Jupyter, VS Code, and PyCharm.
See `truss/notebooks/geometry_quickstart.ipynb` for an interactive example.
