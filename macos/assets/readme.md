# Assets Directory

This directory should contain the application icon files for macOS.

## Required Files

### icon.icns
- **Format**: macOS Icon file (.icns)
- **Required sizes**: 16x16, 32x32, 64x64, 128x128, 256x256, 512x512, 1024x1024
- **Purpose**: Application icon shown in Finder, Launchpad, and app switcher

### icon.png
- **Format**: PNG with transparency
- **Size**: 32x32 pixels (16pt @2x for Retina displays)
- **Purpose**: Menu bar icon
- **Design notes**: Should be monochrome or work well in light/dark mode

## Creating Icons

### Option 1: Using iconutil (Recommended)
1. Create a folder named `icon.iconset`
2. Add PNG files with these exact names:
   ```
   icon_16x16.png
   icon_16x16@2x.png      (32x32)
   icon_32x32.png
   icon_32x32@2x.png      (64x64)
   icon_128x128.png
   icon_128x128@2x.png    (256x256)
   icon_256x256.png
   icon_256x256@2x.png    (512x512)
   icon_512x512.png
   icon_512x512@2x.png    (1024x1024)
   ```
3. Run: `iconutil -c icns icon.iconset -o icon.icns`

### Option 2: Using the build script
If you have a high-resolution `icon.png` (1024x1024), the build script will generate the .icns file:
```bash
./build.sh
```

### Option 3: Using online tools
- [CloudConvert](https://cloudconvert.com/png-to-icns)
- [iConvert Icons](https://iconverticons.com/online/)

## Design Guidelines

### App Icon (icon.icns)
- Use the macOS design language
- Include appropriate shadows and depth
- Consider both light and dark appearances
- Follow [Apple's Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/app-icons)

### Menu Bar Icon (icon.png)
- Use a simple, recognizable silhouette
- Design at 16pt (32x32px for @2x)
- Use template images (black with alpha) for automatic styling
- Test visibility on both light and dark menu bars
- Consider using SF Symbols for consistency

## Template Menu Bar Icon

For best macOS integration, the menu bar PNG should be:
- Black pixels with varying alpha for shading
- Transparent background
- No color information

This allows macOS to automatically adjust the appearance for:
- Light/dark mode
- Selected/pressed states
- Accessibility settings

## Testing Icons

1. **App Icon**: Check appearance in Finder (all view sizes), Dock, and Mission Control
2. **Menu Bar Icon**: Test on both light and dark menu bars
3. **Retina Display**: Ensure @2x versions look crisp
4. **Quick Look**: Select the .icns file and press Space to preview all sizes