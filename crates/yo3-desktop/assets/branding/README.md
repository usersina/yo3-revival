# Launcher icon

The Duel Disk with a lower-right Monster Reborn badge identifies the Revival launcher. Created with the built-in
imagegen tool, using the installed game's icon as the edit target and
[Monster Reborn artwork](https://ygoprodeck.com/card/monster-reborn-7027) as a supporting reference.
The original extracted game assets are not included here.

- `launcher-icon.png`: generated master.
- `launcher-window.png`: 256×256 RGBA8 PNG, decoded at build time for Iced's native window icon.
- `launcher.ico`: Windows executable icon, with 16, 20, 24, 32, 40, 48, 64, 128 and 256-pixel images.

Export the master with ImageMagick:

```sh
magick launcher-icon.png -resize 256x256 -alpha on PNG32:launcher-window.png
magick launcher-icon.png -define icon:auto-resize=256,128,64,48,40,32,24,20,16 launcher.ico
```

The app also draws `launcher-window.png` itself (sidebar, Normal header, About), from the same decoded pixels.
The build checks the window image's dimensions and pixel format. Windows resource compilation embeds the ICO in
`yo3-revival.exe`; the PNG is embedded in the application, so neither needs to be copied beside the executable.

## Generation prompt

Use case: compositing. Asset type: final square desktop application icon for YO3 Revival, 1024x1024. Edit target
Image 1 is the original Yu-Gi-Oh! ONLINE 3 application icon: a white/silver Duel Disk diagonally across a saturated
blue circuit background, cyan circular circuitry in upper left, and a fan of brown-backed trading cards along the
lower edge. Preserve that original icon's composition, colors, object identities, background and proportions
faithfully, reconstructing crisply at high resolution. Supporting Image 2 shows the Monster Reborn amulet. Add ONE
small badge of that recognizable silver winged Monster Reborn amulet with its bright RED oval central gemstone
and purple-blue hanging crystal, as a clean cutout without Image 2's rectangular blue background. Position the badge
anchored in the LOWER RIGHT corner, visually contained inside the square with about 3% edge padding, occupying
roughly the lower-right 38% of the square in width and 42% in height. It may overlap the outermost brown-backed cards
but must leave the main Duel Disk recognizable and dominant. Preserve the amulet's distinctive winged silhouette,
red gem, and pointed blue-purple pendant. Give the added amulet a narrow dark navy separation outline and a subtle
cyan edge highlight, so it reads clearly over the card backs at 32 and 48 pixel icon sizes. Consistent polished
classic game-icon rendering, crisp bold edges rather than photographic texture. Keep the full square blue
background from Image 1. No extra words, no labels, no typography, no additional badges, no drop-shadow outside
the square, no mockup, no border, no UI, no sheet of variants. Output only the single finished icon.
