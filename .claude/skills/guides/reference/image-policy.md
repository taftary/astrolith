# Allowed image formats

The agent may create, update, move, rename, and remove image files inside `guides/` using these extensions:

- `.svg`
- `.png`
- `.jpg`
- `.jpeg`
- `.webp`
- `.gif`
- `.avif`

Use the most appropriate format for the content. Prefer SVG for technical diagrams, geometric constructions, coordinate systems, graphs and node relationships, flowcharts, architecture diagrams, state diagrams, scalable illustrations, and before-and-after refactoring visuals.

Use PNG for screenshots requiring lossless quality, images with transparency, and pixel-accurate technical captures.

Use JPG or JPEG for photographs, large natural images, and images where transparency is not required.

Use WebP for compressed web-oriented illustrations and raster images where smaller file size is useful.

Use GIF only when simple animation is required, and only when animation is necessary.

Use AVIF when high compression and modern browser support are appropriate.

Do not use an image format solely because it is available. Choose the format based on visual content, quality, transparency, scalability, animation, and file size. The agent should not convert an existing image merely to change its format unless there is a clear benefit.

## Image licensing and attribution

Only public-domain or permissively licensed images may be copied into `guides/`. Record the origin and the license of every copied image in the topic's `sources.md`. When the license is unclear, restrictive, or unknown, link to the image instead of copying it. Prefer drawing the visual as SVG over copying a raster image when the content can be expressed as a diagram.

## Diagram and image management

Create diagrams when they improve understanding, especially for geometric constructions, coordinate systems, graphs and node relationships, hierarchies, algorithms, data flows, architecture, state transitions, comparisons, before-and-after refactors, and contradictions or ambiguous structures.

Every image must have a descriptive lowercase hyphenated filename, meaningful alt text, a caption when useful, a link from at least one relevant Markdown document, and a generation or source note when relevant. Example: `*Figure 1. Classification of segment-triangle intersection cases.*`

Do not create decorative images that do not improve comprehension. When a specification changes, inspect related images and update them if they are no longer accurate.

# Image validation

For every image, validate:

- The file uses an allowed image extension.
- The image exists at the referenced path.
- The Markdown reference uses the correct relative path.
- The image can be opened or parsed when validation is available.
- The image has meaningful alt text.
- The filename is descriptive and lowercase with hyphens.
- The image is linked from at least one relevant document.
- The image format is appropriate for its content.
- The image is not a duplicate of another image.
- The image is not obsolete.
- The image is not unnecessarily converted from a suitable existing format.
- Any affected references are updated after a rename, move, replacement, or deletion.
