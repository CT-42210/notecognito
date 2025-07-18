# Assets Directory

This directory should contain the application icon file.

## Required File

### icon.ico
- **Format**: Windows Icon file (.ico)
- **Recommended sizes**: Include 16x16, 32x32, 48x48, and 256x256 pixels
- **Color depth**: 32-bit with alpha channel for transparency
- **Purpose**: Used for system tray icon and application icon

## Creating the Icon

### Option 1: Using an online converter
1. Create a 256x256 PNG image with transparency
2. Use a service like https://convertio.co/png-ico/
3. Save as `icon.ico` in this directory

### Option 2: Using ImageMagick
```bash
# If you have multiple PNG files of different sizes:
magick convert icon-16.png icon-32.png icon-48.png icon-256.png icon.ico

# Or from a single high-resolution PNG:
magick convert icon-256.png -define icon:auto-resize=256,48,32,16 icon.ico
```

### Option 3: Using Visual Studio
1. Add a new Icon resource to a project
2. Draw or import your icon at multiple resolutions
3. Save as `icon.ico`

## Icon Design Guidelines

- Use a clear, simple design that's recognizable at 16x16
- Include transparency for a professional look
- Consider using the notecard/sticky note metaphor
- Test visibility on both light and dark backgrounds
- Ensure good contrast for the system tray

## Placeholder

Until you create a proper icon, you can use any `.ico` file renamed to `icon.ico` for testing purposes.