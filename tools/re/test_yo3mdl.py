"""tools/re/yo3mdl.py: a model built here (no game data) encodes, parses back to the same tree, and re-encodes to the same
bytes; the game's own models are checked by `python3 tools/re/yo3mdl.py data/model/*.mdl` where the game is."""
import importlib.util, struct, unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('yo3mdl', Path(__file__).with_name('yo3mdl.py'))
mdl = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(mdl)

def floats(*v): return struct.pack('<%df' % len(v), *v)
IDENTITY = floats(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)

def tiny():
    """One triangle and one quad (7 corners), UVs, a material list, one joint's weights, one animation with a camera."""
    material = dict(face=bytes([255, 255, 255, 255]), power=floats(1), specular=bytes(3), emissive=bytes(3), textures=[b'.\\\\doll.png'])
    mesh = dict(name=b'body', pos=floats(*range(21)), face_ctl=[3, 4], face_sizes=[3, 4], sub=[
        (0x16, floats(*([0.5] * 14))),
        (0x1a, dict(nmat=1, k=2, idx=None, mats=[('ref', b'body_Material')])),
        (0x3c, (1, 1, 1)),
        (0x46, dict(name=b'root', n=7, idruns=[(0, 7)], vals=[floats(1)] * 7, vruns=[(0, 7)], mat=IDENTITY)),
    ])
    root = dict(name=b'root', items=[(0xb, IDENTITY), (0x14, mesh), (10, dict(name=b'child', items=[(0xc, None)]))])
    animation = dict(name=b'idle', items=[
        (0x32, [(0, [struct.pack('<H', 1) + floats(0, 100, -300)]), (2, [struct.pack('<H', 1) + floats(0.7)])]),
        (0x29, b'root', [(0, [struct.pack('<H', 1) + floats(-1, 0, 0, 0)]), (9, [struct.pack('<HB', 1, 1)])]),
    ])
    return [(1, 30), (2, (b'body_Material', material)), (10, root), (0x28, animation), (0xff, None)]

class Yo3Mdl(unittest.TestCase):
    def test_a_model_written_here_reads_back_the_same(self):
        data = mdl.encode(tiny())
        self.assertTrue(data.startswith(mdl.MAGIC) and data.endswith(b'\xff'))
        tree = mdl.parse(data)
        self.assertEqual(tree, tiny())
        self.assertEqual(mdl.encode(tree), data)

    def test_faces_are_runs_of_sizes_and_weights_runs_of_values(self):
        # 0x13: a run of 3 faces of 3 corners (run - 1 in the upper bits); 0x82: two weights each stored.
        model = tiny()
        mesh = model[2][1]['items'][1][1]
        mesh.update(pos=floats(*range(27)), face_ctl=[0x13], face_sizes=[3, 3, 3])
        mesh['sub'][3][1].update(n=9, idruns=[(0, 9)], vals=[floats(1)] * 9, vruns=[(0, 9)])
        data = mdl.encode(model)
        self.assertEqual(mdl.parse(data)[2][1]['items'][1][1]['face_sizes'], [3, 3, 3])
        with self.assertRaises(AssertionError):  # a face count that isn't the corners'
            mesh['face_sizes'] = [3, 3]; mesh['face_ctl'] = [0x0b]; mdl.parse(mdl.encode(model))

if __name__ == '__main__':
    unittest.main()
