"""Trellis algorithms and systems framework.

High-performance algorithms, geometry processing, and systems framework
providing unified Rust and Python APIs.
"""

from __future__ import annotations

from trellis._trellis import (
    GeometryAsset,
    GeometryService,
    Session,
    load_geometry,
    version,
)

__version__: str = version()

__all__: list[str] = [
    "GeometryAsset",
    "GeometryService",
    "Session",
    "load_geometry",
    "version",
]
