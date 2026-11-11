# Synthetic guarded sites, never original game code. The callback pointer gets a real PE base relocation.
.section .yo3login,"xr"
.byte 0xe8,0x50,0x6c,0x08,0x00
.section .yo3starter,"xr"
.byte 0xe8,0x3e,0x05,0x00,0x00
.section .yo3duel,"xr"
.byte 0xe8,0x79,0x00,0x00,0x00
.section .yo3update,"xr"
.byte 0xb8
.long _yo3_fixture_update
.section .yo3target,"xr"
_yo3_fixture_update:
ret
