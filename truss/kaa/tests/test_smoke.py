# truss/kaa/tests/test_smoke.py -------------------------------------------------------------

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

    def test_fresco_example(self):
        import trellis

        uri = "example://Fresco/MathView"
        document = trellis.open(uri)
        self.assertIsInstance(document, trellis.Document)
        info = trellis.inspect(document, root="Matrix_A", coordinates=(1, 0))
        self.assertIsInstance(info, trellis.InspectionInfo)
        self.assertEqual(info.name, "Matrix_A")
        self.assertIn("Var(A[1,0])", info.selected_term)

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

    def test_fresco_tensor_numpy_and_solver(self):
        import numpy as np
        import trellis

        ctx = trellis.FrescoContext()
        src = np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float64)
        a = ctx.tensor_from_numpy(src)
        src[0, 0] = 999.0  # Copy semantics: source mutation must not affect Tensor
        v = ctx.tensor_from_list([2], [10.0, 20.0])

        mv = a @ v
        self.assertEqual(mv.shape, (2,))
        arr = mv.to_numpy()
        self.assertTrue(np.allclose(arr, np.array([50.0, 110.0])))

        # Cross-context handle rejection
        other_ctx = trellis.FrescoContext()
        other_v = other_ctx.tensor_from_list([2], [1.0, 1.0])
        with self.assertRaises(ValueError):
            _ = a @ other_v

        # Least-squares Rosenbrock solve across backends
        r0 = ctx.parse("10 * (y - x^2)")
        r1 = ctx.parse("1 - x")
        for backend in ("symbolic", "forward_ad", "reverse_ad"):
            solver = ctx.prepare_least_squares([r0, r1], ["x", "y"], backend=backend)
            diag = solver.solve([-1.2, 1.0])
            self.assertTrue(diag.converged)
            self.assertEqual(diag.status, "ResidualSatisfied")
            self.assertTrue(np.allclose(diag.best_x_numpy(), np.array([1.0, 1.0]), atol=1e-8))


if __name__ == "__main__":
    unittest.main()

