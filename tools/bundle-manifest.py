#!/usr/bin/env python3
"""Inventory the executable components of a release, never game data or user credentials."""
import hashlib,json,os,re,subprocess,sys
from pathlib import Path
out=Path(sys.argv[1]); edition=sys.argv[2]; assert edition in ('desktop','scripts'); windows=(out/'yo3-server.exe').is_file(); ext='.exe' if windows else ''
source=Path('crates/yo3-tunnel/src/lib.rs').read_text()
number=lambda name:int(re.search(rf'pub const {name}: u16 = (\d+);',source)[1])
roles={'server':'yo3-server'+ext,'launcher':'yo3-launcher'+ext,
       'injector':'yo3-auth-launch.exe','connection_mod':'yo3-auth.dll'}
if edition=='desktop': roles.update(desktop='yo3-revival'+ext,loader='yo3-bootstrap.dll')
if (out/'yo3-launcher-windows.exe').is_file(): roles['direct_launcher']='yo3-launcher-windows.exe'
components={role:dict(file=name,sha256=hashlib.sha256((out/name).read_bytes()).hexdigest()) for role,name in roles.items()}
manifest=dict(schema=1,edition=edition,release=os.environ['YO3_BUILD_VERSION'],commit=os.environ.get('YO3_BUILD_COMMIT') or subprocess.check_output(['git','rev-parse','--short=12','HEAD'],text=True).strip(),
              protocol=number('PROTOCOL_VERSION'),mod_abi=number('MOD_ABI'),components=components)
(out/'bundle.json').write_text(json.dumps(manifest,indent=2)+'\n')
