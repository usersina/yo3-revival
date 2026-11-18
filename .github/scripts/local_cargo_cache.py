#!/usr/bin/env python3
"""Keep this trusted repository's Cargo output on its dedicated home runner, without network archives."""
from pathlib import Path
import shutil
import subprocess

OWNER = 'yo3-revival Cargo cache v1\n'
MAX_BYTES = 40 * 1024**3
MIN_FREE = 20 * 1024**3


def prepare(repo: Path, home: Path) -> str:
    cache = home / '.cache/yo3-revival/cargo-target'
    marker = cache / '.yo3-cache-owner'
    target = repo / 'target'
    if target.is_symlink():
        if target.resolve() != cache.resolve():
            raise ValueError('target points outside this repository’s Cargo cache')
    elif target.exists():
        raise ValueError('Refusing to replace an existing target directory; run checkout cleanup first')
    if cache.is_symlink() or cache.parent.is_symlink():
        raise ValueError('The Cargo cache must be a real directory')
    cache.mkdir(parents=True, exist_ok=True)
    if marker.is_symlink() or (any(cache.iterdir()) and (not marker.is_file() or marker.read_text() != OWNER)):
        raise ValueError('Refusing to clear an unowned Cargo cache')
    marker.write_text(OWNER)
    size = int(subprocess.check_output(['du', '-sk', '--', str(cache)], text=True).split()[0]) * 1024
    free = shutil.disk_usage(cache).free
    reset = size > MAX_BYTES or free < MIN_FREE
    if reset:
        shutil.rmtree(cache)
        cache.mkdir()
        marker.write_text(OWNER)
        if shutil.disk_usage(cache).free < MIN_FREE:
            raise RuntimeError("Insufficient free disk space after clearing this repository’s Cargo cache (need 20 GiB)")
    if not target.is_symlink():
        target.symlink_to(cache, target_is_directory=True)
    return f'Cargo cache {"reset" if reset else "reused"}: {cache} ({size // 1024**2} MiB before preparation; {free // 1024**3} GiB free)'


if __name__ == '__main__':
    print(prepare(Path.cwd(), Path.home()))
