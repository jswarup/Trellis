# python/tests/test_smoke.py ------------------------------------------------------------------

"""Smoke tests for the installed `trellis` Python package."""

import unittest


class TestTrellisPackage(unittest.TestCase):
    def test_import_and_version(self):
        import trellis

        self.assertTrue(hasattr(trellis, "__version__"))
        self.assertTrue(hasattr(trellis, "version"))
        self.assertEqual(trellis.version(), trellis.__version__)
        self.assertRegex(trellis.__version__, r"^\d+\.\d+\.\d+")

    def test_session_creation(self):
        import trellis

        session = trellis.Session()
        self.assertIn("Trellis Session", repr(session))
        self.assertEqual(session.version(), trellis.__version__)

    def test_parse_obj(self):
        import trellis

        session = trellis.Session()
        obj_data = (
            "v 0.0 0.0 0.0\n"
            "v 1.0 0.0 0.0\n"
            "v 0.0 1.0 0.0\n"
            "f 1 2 3\n"
        )
        asset = session.parse_obj(obj_data)
        self.assertEqual(asset.vertex_count, 3)
        self.assertEqual(asset.face_count, 1)
        self.assertFalse(asset.is_point_cloud)

        positions = asset.vertex_positions()
        self.assertEqual(len(positions), 3)

        faces = asset.faces()
        self.assertEqual(len(faces), 1)
        self.assertEqual(faces[0], [0, 1, 2])

    def test_parse_pts(self):
        import trellis

        session = trellis.Session()
        pts_data = (
            "2\n"
            "1.0 2.0 3.0 200 255 0 0\n"
            "4.0 5.0 6.0 100 0 255 0\n"
        )
        asset = session.parse_pts(pts_data)
        self.assertEqual(asset.point_count, 2)
        self.assertTrue(asset.is_point_cloud)

        colors = asset.vertex_colors()
        self.assertEqual(len(colors), 2)

    def test_error_handling(self):
        import trellis

        session = trellis.Session()
        with self.assertRaises(FileNotFoundError):
            session.load_geometry("non_existent_file_xyz_123.obj")

        with self.assertRaises(ValueError):
            session.parse_obj("corrupted obj content")


if __name__ == "__main__":
    unittest.main()

