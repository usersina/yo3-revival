/* Keep PE sections contiguous: Windows rejects the gaps accepted by Wine's image loader. */
SECTIONS {
  /* MinGW CRT code size varies by toolchain. Follow .text while preserving every guarded RVA. */
  __yo3_hook_start = ALIGN(ADDR(.text) + SIZEOF(.text), __section_alignment__);
  .yo3hook __yo3_hook_start : SUBALIGN(1) {
    . = __image_base__ + 0x6964b - __yo3_hook_start; KEEP(*(.yo3login))
    . = __image_base__ + 0xf44bd - __yo3_hook_start; KEEP(*(.yo3starter))
    . = __image_base__ + 0x114212 - __yo3_hook_start; KEEP(*(.yo3duel))
    . = __image_base__ + 0x1159ea - __yo3_hook_start; KEEP(*(.yo3update))
    . = __image_base__ + 0x115a30 - __yo3_hook_start; KEEP(*(.yo3target))
    . = __image_base__ + 0x116000 - __yo3_hook_start;
  }
  ASSERT(__yo3_hook_start <= __image_base__ + 0x6964b, "fixture .text overlaps its first guarded hook")
} INSERT AFTER .text;
