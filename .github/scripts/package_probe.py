#!/usr/bin/env python3
"""Publish a tiny disposable version, then prove the job token can delete exactly that version."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
from release_lifecycle import GitHub, delete_probe

def main():
    api=GitHub(os.environ['GITHUB_REPOSITORY'])
    tag=f'deletion-probe-{os.environ["GITHUB_RUN_ID"]}-{os.environ["GITHUB_RUN_ATTEMPT"]}'
    if not re.fullmatch(r'deletion-probe-[0-9]+-[0-9]+',tag):raise ValueError('Invalid CI run identity')
    with tempfile.TemporaryDirectory(prefix='yo3-package-probe-') as directory:
        root=Path(directory)
        (root/'probe').write_text(tag+'\n')
        (root/'Dockerfile').write_text(f'FROM scratch\nCOPY probe /probe\nLABEL org.opencontainers.image.source="https://github.com/{api.repo}"\n')
        metadata=root/'metadata.json'
        subprocess.run(['docker','buildx','build','--push','--provenance=false','--platform','linux/amd64',
            '--metadata-file',str(metadata),'-t',f'ghcr.io/{api.repo}-dashboard:{tag}',str(root)],check=True,timeout=300)
        delete_probe(api,json.loads(metadata.read_text())['containerimage.digest'],tag)

if __name__=='__main__':main()
