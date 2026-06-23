# TexSwizzle
## About
Swizzle textures into composite Helldivers 2 textures.

Use the --metallic, --normal, --roughness, --ao, and --emissive options to provide textures that will be swizzled. Use --basic or --advanced to produce a basic or advanced pbr respectively. 

Any required texture that is omitted will instead use a sensible default. For example, omitting the --normal option when creating an --advanced PBR will behave as if an all-flat normal was provided. All-flat means \[128, 128, 255] pixel values.

Textures do not have to be the same size. The output will be the same dimensions
as the largest input, and the smaller inputs will be upscaled using the 'nearest'
sampling method.

## Why?
Most people use GIMP to do this by manually aligning the channels and then merging them there. This is a waste of time for anyone making a large number of textures. This simplifies that process.