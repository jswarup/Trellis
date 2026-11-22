"""Trellis algorithms and systems framework.

High-performance algorithms, geometry processing, and systems framework
providing unified Rust and Python APIs.
"""

from __future__ import annotations

from trellis._trellis import (
    Document,
    GeometryAsset,
    GeometryService,
    InspectionInfo,
    Job,
    Session,
    Settings,
    inspect,
    load_geometry,
    open,
    run,
    version,
)

__version__: str = version()

__all__: list[str] = [
    "Document",
    "GeometryAsset",
    "GeometryService",
    "InspectionInfo",
    "Job",
    "Session",
    "Settings",
    "inspect",
    "load_geometry",
    "open",
    "run",
    "version",
]
