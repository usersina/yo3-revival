#!/usr/bin/env python3
"""Keep this trusted repository's Cargo output on its dedicated home runner, without network archives."""
from pathlib import Path
import re
import shutil
import subprocess

OWNER = 'yo3-revival Cargo cache v1\n'
MAX_BYTES = 40 * 1024**3
# A release run from a cold cache builds about 40 GiB (2026-11-24, measured on home-yo3), on a VM shared with other
# projects' runners: start only with room for that.
MIN_FREE = 45 * 1024**3
NAME = re.compile(r'^name\s*=\s*"([^"]+)"', re.M)


def workspace_packages(repo: Path) -> set[str]:
    """This repository's own crates (crates/*/Cargo.toml): their builds change with nearly every commit."""
    names = set()
    for manifest in repo.glob('crates/*/Cargo.toml'):
        m = NAME.search(manifest.read_text(encoding='utf-8', errors='replace'))
        if m:
            names.add(m.group(1))
    return names


def prune(cache: Path, packages: set[str]) -> int:
    """Removes every earlier build of this repository's own crates (Cargo's `<profile>/build/<package>/<hash>`, for the host
    and the Windows target): a new commit rebuilds them anyway, and kept they piled up (223 builds of yo3-server, 33 of
    37 GiB, until the disk filled mid-run). Third-party crates, the part worth keeping, stay. Bytes removed."""
    removed = 0
    for build in list(cache.glob('*/build')) + list(cache.glob('*/*/build')):
        for package in packages:
            folder = build / package
            if folder.is_dir() and not folder.is_symlink():
                removed += int(subprocess.check_output(['du', '-sk', '--', str(folder)], text=True).split()[0]) * 1024
                shutil.rmtree(folder)
    return removed


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
    pruned = prune(cache, workspace_packages(repo))
    size = int(subprocess.check_output(['du', '-sk', '--', str(cache)], text=True).split()[0]) * 1024
    free = shutil.disk_usage(cache).free
    reset = size > MAX_BYTES or free < MIN_FREE
    if reset:
        shutil.rmtree(cache)
        cache.mkdir()
        marker.write_text(OWNER)
        if shutil.disk_usage(cache).free < MIN_FREE:
            raise RuntimeError("Insufficient free disk space after clearing this repository’s Cargo cache (need 45 GiB)")
    if not target.is_symlink():
        target.symlink_to(cache, target_is_directory=True)
    return (f'Cargo cache {"reset" if reset else "reused"}: {cache} ({pruned // 1024**2} MiB of this repository\'s earlier builds '
            f'pruned, {size // 1024**2} MiB then; {free // 1024**3} GiB free)')


if __name__ == '__main__':
    print(prepare(Path.cwd(), Path.home()))
