/* Keep PE sections contiguous: Windows rejects the gaps accepted by Wine's image loader. */
SECTIONS {
  .yo3hook __image_base__ + 0x4000 : SUBALIGN(1) {
    . = 0x6964b - 0x4000; KEEP(*(.yo3login))
    . = 0xf44bd - 0x4000; KEEP(*(.yo3starter))
    . = 0x114212 - 0x4000; KEEP(*(.yo3duel))
    . = 0x1159ea - 0x4000; KEEP(*(.yo3update))
    . = 0x115a30 - 0x4000; KEEP(*(.yo3target))
    . = 0x116000 - 0x4000;
  }
  ASSERT(SIZEOF(.text) <= 0x3000, "fixture .text grew past its reserved image range")
} INSERT AFTER .text;
