#!/usr/bin/env python3
"""vm.py without a VM: configuration, coordinates, keys, the guest batch file, PNG reading and `run`'s round trip."""
import importlib.util
import struct
import tempfile
import threading
import time
import unittest
import zlib
from pathlib import Path
from unittest import mock

SPEC = importlib.util.spec_from_file_location('vm', Path(__file__).with_name('vm.py'))
vm = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(vm)


def png(width, height, pixel, filter_type=1):
    """A width x height RGB PNG of one colour, rows written with `filter_type` (1 = Sub, 2 = Up)."""
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    rows = b''
    for y in range(height):
        if filter_type == 1:
            line = bytes(pixel) + bytes(3 * (width - 1))
        else:
            line = bytes(pixel) * width if y == 0 else bytes(3 * width)
        rows += bytes([filter_type]) + line
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


class Config(unittest.TestCase):
    def test_defaults_then_the_repository_then_the_user_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo, user = Path(tmp, 'repo.toml'), Path(tmp, 'user.toml')
            repo.write_text('[defaults]\nconnect = "qemu:///system"\nshare = "/data/shared"\nguest_share = "Z:"\n'
                            '[win7]\ndescription = "seven"\n')
            user.write_text('[defaults]\nshare = "/mnt/vms"\n[win7]\nssh = "me@win7"\n[win8]\ndescription = "new"\n')
            config = vm.load_config((repo, user))
        self.assertEqual(config['win7'] | {}, {'connect': 'qemu:///system', 'share': '/mnt/vms', 'guest_share': 'Z:',
                                               'description': 'seven', 'ssh': 'me@win7', 'name': 'win7'})
        self.assertEqual(config['win8']['share'], '/mnt/vms')

    def test_the_repository_file_parses_and_an_unknown_vm_is_named(self):
        config = vm.load_config((Path(vm.__file__).with_name('vms.toml'),))
        self.assertIn('win11', config)
        with self.assertRaisesRegex(vm.Fail, 'no VM .nope.'):
            vm.vm_config('nope', config)


class Input(unittest.TestCase):
    def test_screen_coordinates_scale_to_the_tablet_and_stay_on_screen(self):
        events = vm.tablet(1023, 0, (1024, 768))
        self.assertEqual([e['data']['value'] for e in events], [32767, 0])
        self.assertEqual(vm.tablet(512, 384, (1024, 768))[1]['data']['value'], round(384 * 32767 / 767))
        with self.assertRaises(vm.Fail):
            vm.tablet(1024, 10, (1024, 768))

    def test_characters_map_to_us_keys(self):
        self.assertEqual(vm.qcodes_for('A'), ['shift', 'a'])
        self.assertEqual(vm.qcodes_for('\\'), ['backslash'])
        self.assertEqual(vm.qcodes_for(':'), ['shift', 'semicolon'])
        with self.assertRaises(vm.Fail):
            vm.qcodes_for('é')

    def test_a_combination_presses_in_order_and_releases_in_reverse(self):
        events = vm.key_events(['ctrl', 'Shift', 'esc'])
        pressed = [(e['data']['down'], e['data']['key']['data']) for e in events]
        self.assertEqual(pressed, [(True, 'ctrl'), (True, 'shift'), (True, 'esc'),
                                   (False, 'esc'), (False, 'shift'), (False, 'ctrl')])
        self.assertEqual(vm.key_events(['Enter'])[0]['data']['key']['data'], 'ret')

    def test_typing_refuses_before_sending_anything(self):
        with mock.patch.object(vm, 'keys') as keys:
            with self.assertRaises(vm.Fail):
                vm.type_text({'name': 'x'}, 'ok é')
        keys.assert_not_called()


class Png(unittest.TestCase):
    def test_size_and_pixels_through_sub_and_up_filters(self):
        for filter_type in (1, 2):
            data = png(4, 3, (10, 200, 30), filter_type)
            self.assertEqual(vm.png_size(data), (4, 3))
            width, height, rows = vm.png_pixels(data)
            self.assertEqual((width, height), (4, 3))
            self.assertEqual(rows[2][9:12].hex(), '0ac81e')

    def test_not_a_png(self):
        with self.assertRaises(vm.Fail):
            vm.png_size(b'P6\n1 1\n255\n\0\0\0')


class Run(unittest.TestCase):
    def test_the_batch_file_writes_output_then_the_code_last(self):
        text = vm.batch({'guest_share': 'Z:'}, 'abc', 'dir C:\\')
        self.assertIn('call :run > "Z:\\.yo3-vm\\abc.out" 2>&1', text)
        self.assertIn('\r\n:run\r\ndir C:\\\r\nexit /b %errorlevel%\r\n', text)
        self.assertLess(text.index('abc.tmp'), text.index('move /y "Z:\\.yo3-vm\\abc.tmp" "Z:\\.yo3-vm\\abc.rc"'))
        self.assertTrue(text.endswith('\r\n'))

    def test_run_types_the_launcher_and_returns_the_guest_result(self):
        with tempfile.TemporaryDirectory() as share:
            config = {'name': 'win7', 'share': share, 'guest_share': 'Z:'}
            typed = []

            def guest():  # what the guest does once the batch file appears
                folder = Path(share, vm.RUN_DIR)
                while not list(folder.glob('*.cmd')):
                    time.sleep(0.05)
                ident = next(folder.glob('*.cmd')).stem
                (folder / f'{ident}.out').write_bytes(b'Microsoft Windows [Version 6.1.7601]\r\n')
                (folder / f'{ident}.rc').write_text('3 \r\n')
            thread = threading.Thread(target=guest)
            thread.start()
            with mock.patch.object(vm, 'keys'), mock.patch.object(vm, 'type_text', lambda _, text: typed.append(text)), \
                    mock.patch.object(vm.time, 'sleep'), mock.patch('sys.stdout') as out:
                code = vm.run(config, 'ver', timeout=10)
            thread.join()
            self.assertEqual(code, 3)
            self.assertRegex(typed[0], r'^cmd /c Z:\\\.yo3-vm\\[0-9a-f]{12}\.cmd$')
            self.assertIn('6.1.7601', ''.join(c.args[0] for c in out.write.call_args_list))
            self.assertEqual(list(Path(share, vm.RUN_DIR).iterdir()), [], 'the files are removed afterwards')

    def test_run_prefers_ssh(self):
        with mock.patch.object(vm.subprocess, 'run', return_value=mock.Mock(returncode=0)) as run:
            self.assertEqual(vm.run({'name': 'w', 'ssh': 'me@w'}, 'ver'), 0)
        self.assertEqual(run.call_args.args[0], ['ssh', '-o', 'BatchMode=yes', 'me@w', 'ver'])


if __name__ == '__main__':
    unittest.main()
