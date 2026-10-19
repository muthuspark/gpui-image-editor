# Minimal GPUI Image Editor PRD

Version 1.0 · Draft · 19 October 2026

## Product goal

Build a lightweight desktop application for everyday image edits using Rust and GPUI. A user should be able to open a screenshot or photograph, crop it, resize it, adjust its appearance, and export a copy in under one minute for a straightforward task.

The initial release is a single window, single image editor. It runs entirely offline, requires no account, and never modifies the source file.

## Users and primary workflow

The primary users are developers, product teams, and everyday desktop users preparing screenshots and photographs for documents, websites, and messages.

1. Open a PNG or JPEG using the file picker or drag and drop.
2. Inspect the image using fit to window, zoom, and pan.
3. Apply crop, resize, rotation, flip, or basic color adjustments.
4. Undo mistakes or reset edits.
5. Export a PNG or JPEG to a chosen location.

Success means this complete workflow works reliably without a tutorial.

## MVP boundaries

The release must include image opening, viewing, geometric edits, brightness and contrast, undo and redo, and safe export.

Exclude layers, brushes, drawing, text, annotations, selections beyond rectangular crop, masks, filters, AI features, RAW files, animation, batch processing, cloud storage, plugins, and editable project files. Do not add these to make an incomplete milestone appear more useful.

Platform is a release decision: ship on one operating system first. Select and record that operating system and minimum supported version during the first milestone, based on the intended users and available test hardware. Keep processing code independent of platform APIs; other operating systems are subsequent releases.

## Functional requirements

| ID | Requirement | Acceptance criteria |
| --- | --- | --- |
| F01 | Open images | File picker and single file drop accept static PNG and JPEG. On success, show filename, oriented pixel dimensions, and image fitted to the canvas. Cancel leaves the current document unchanged. |
| F02 | Validate input | Reject corrupt, unsupported, or oversized files with an actionable message. Check dimensions and decoded allocation limits before allocating the full image. A failed open preserves the existing document and edits. |
| F03 | Navigate canvas | Support fit, 100% view, zoom from 10% to 800%, and space plus drag to pan. Pointer zoom keeps the image point under the cursor stable where possible. Navigation never enters edit history. |
| F04 | Crop | Draw and adjust a rectangular crop, with freeform and 1:1 modes. Show output dimensions. Enter or Apply commits; Escape or Cancel discards the pending crop. Clamp to image bounds and require at least one pixel in each dimension. |
| F05 | Resize | Accept positive integer width and height, with aspect ratio locked by default. Changing one dimension computes the other, rounded to the nearest pixel. Show resulting dimensions before Apply; reject values beyond supported limits. |
| F06 | Rotate and flip | Provide 90° clockwise, 90° counterclockwise, horizontal flip, and vertical flip. A 90° rotation swaps dimensions. Four clockwise rotations restore identical pixels. |
| F07 | Adjust appearance | Provide brightness and contrast sliders from −100 to +100, default 0. Preview while dragging; one complete drag is one undo action. Provide per-control reset. Define and document deterministic pixel transforms before implementation. |
| F08 | Undo and redo | Support at least 50 committed editing actions. Undo restores exact prior edit state; redo restores the reverted state. A new edit after undo clears the redo branch. Disable unavailable actions. |
| F09 | Reset | Reset All creates one undoable action restoring the opened image after orientation normalization. |
| F10 | Export | Export to PNG or JPEG with filename and destination selection. JPEG quality defaults to 90 and is adjustable from 1 to 100. Show final dimensions and format before writing. |
| F11 | Protect files | Default filename is source-name-edited. Reject export to the source path, including aliases resolving to that file. Confirm before replacing any other existing file. Failed export leaves that destination intact. |
| F12 | Protect pending work | Before close, quit, or opening another image with unexported edits, offer Export, Discard, or Cancel. Only a successful export of the current revision clears its unexported status. Canceling or failing export cancels the pending destructive action. |

### Format and image fidelity

Use an 8-bit RGBA working image. Normalize JPEG EXIF orientation once at import; subsequent dimensions and coordinates refer to this normalized image. Preserve PNG alpha through edits and PNG export. JPEG export composites transparency onto white, with this behavior stated in the export dialog.

For a deliberately small first release, accept untagged and sRGB images. Detect and reject images with unsupported color profiles or precision rather than silently misrepresenting them. Strip EXIF, GPS, and other source metadata on export; encode output as sRGB with orientation already applied. Professional color management and metadata preservation are outside the MVP.

Support images up to 24 megapixels, a maximum side of 12,000 pixels, and an encoded file size of 100 MiB. These are concurrent limits, including resized output. Reject animated PNG and unsupported pixel encodings with a clear explanation.

### Editing behavior

Committed operations execute in user order. Adjustments preview against the current committed result; Apply commits both adjustment values together and returns the controls to neutral. Cancel discards that preview. Applying a neutral adjustment makes no history entry.

Crop and adjustment previews are temporary states. Switching tools requires Apply or Cancel when a preview would change pixels. Undo and export operate on committed states; the interface resolves any pending preview first.

Export captures one immutable revision. If the user edits while export is running, the successful export does not mark the newer revision as exported. Export never changes the opened source or discards undo history. Exported images are flattened; edit history exists only for the current session.

## Interface and interaction

Use a neutral dark interface with one main window and a minimum usable size of 960 × 640 logical pixels.

| Area | Contents |
| --- | --- |
| Top bar | Open, Undo, Redo, Reset All, Export, filename, and unexported indicator |
| Left toolbar | Pan, Crop, Resize, Rotate, Flip, Adjust |
| Center | Image canvas with checkerboard transparency and crop overlay |
| Right panel | Controls for the selected tool, Apply, and Cancel |
| Status bar | Current output dimensions, zoom percentage, and operation status |

The empty state says “Open an image or drop one here.” Disable editing controls until an image loads. Show busy and error states inline without replacing the canvas. Long operations must not freeze input. On multiple file drop, explain that only one image can be opened at a time.

Use platform-standard modifiers for Open, Export, Undo, and Redo. Escape cancels a preview; Enter applies a tool outside text inputs. Provide tooltips, visible keyboard focus, labeled controls, and keyboard access to all toolbar and dialog actions. User-facing errors explain the failed action and how to recover.

## Performance and reliability

The following are release targets, not claims about existing GPUI performance. During the first milestone, record one reference machine, operating system, GPU, display resolution, and release build configuration. Use that same environment for acceptance measurements.

| Measurement | Target |
| --- | --- |
| Cold launch to interactive empty window | p95 ≤ 2 seconds across 20 launches |
| Open 12 MP JPEG from local SSD to first usable image | p95 ≤ 1 second across 20 opens |
| Pan and zoom after load | p95 frame time ≤ 16.7 ms during a 10 second interaction on a 60 Hz display |
| Adjustment interaction | Updated reduced-resolution preview within 100 ms at p95; full-resolution result within 2 seconds after Apply for 12 MP |
| Export 12 MP image to PNG or JPEG | p95 ≤ 5 seconds per format across 20 runs |
| Memory | Peak process resident memory ≤ 1 GiB during the 24 MP acceptance workflow, including 50 edits and one export |
| Stability | No crashes, source modifications, or destination corruption in the release test suite |

Maintain a responsive interface even if an operation exceeds its latency target. Display progress or a busy indicator within 100 ms. Publish actual measured results alongside targets; do not claim speed or memory advantages over other editors without equivalent benchmarks.

## Technical constraints and implementation direction

Rust is the application language. GPUI owns the window, layout, input, and canvas presentation. GPUI is a GPU-accelerated UI framework; image transformation throughput must be measured independently. Do not assume that using GPUI automatically moves image processing onto the GPU.

Keep four responsibilities separate: document and history state, deterministic image processing, GPUI views and interaction state, and file decoding and export. A suitable processing library may be selected during the initial technical spike and pinned with GPUI to tested versions.

Store an immutable normalized source plus an ordered edit history and cursor. Avoid storing a full-resolution bitmap for every undo step. Use bounded checkpoints and evictable preview caches to meet the history and memory requirements together.

Perform decoding, transforms, and encoding on background workers. Tag results with document and revision identifiers; discard stale results so an older computation cannot overwrite newer edits. Coalesce slider updates and bound queued work. Keep expensive pixel work out of render callbacks.

Use the same transformation semantics for preview and full-resolution export. Map pointer input through zoom, pan, and display scaling into image coordinates; crop coordinates must remain accurate at non-100% zoom.

Write exports to a temporary file in the destination directory and atomically replace the destination after successful encoding. On failure, preserve the previous file and clean up temporary output. Do not send images, paths, or usage data over the network. The MVP includes no telemetry or automatic update service.

## Delivery milestones

1. **Platform and rendering spike.** Select the first operating system, pin dependencies, open a window, decode and display representative images, verify file dialogs and image upload to GPUI, and establish performance baselines. Exit when the selected platform can support the complete workflow.
2. **Viewer and document lifecycle.** Implement opening, validation, zoom, pan, errors, and safe document replacement. Exit when malformed files and canceled opens preserve existing work.
3. **Editing and history.** Implement crop, resize, rotation, flip, adjustments, and bounded undo and redo. Exit when deterministic image fixtures verify pixels, dimensions, and history behavior.
4. **Export and release.** Implement encoding, file protection, pending-work prompts, keyboard operation, packaging, and benchmarks. Exit when the release gates below pass.

Each milestone must leave a runnable application. Build only capabilities required for the next milestone; do not create a plugin framework or generalized graphics engine.

## Verification and release gates

Use small asymmetric pixel fixtures to verify rotation, flips, crop coordinates, alpha, and undo exactly. For resize and color adjustments, use versioned reference outputs with documented interpolation, formulas, and tolerances. JPEG comparisons use a documented error tolerance because encoding is lossy.

Cover corrupt headers, decoded allocation limits, EXIF orientations, transparent PNG, unsupported profiles, Unicode paths, read-only destinations, disk-full failures, overwrite cancellation, and canceled file dialogs. Include tests for stale background results and edits made during export.

Run the packaged release build through this end-to-end scenario: open an oriented JPEG, crop, resize, rotate, change brightness and contrast, undo and redo, export PNG, reopen it, and verify dimensions and expected pixels. Repeat with transparent PNG and JPEG export onto white.

Release only when all F01–F12 acceptance criteria pass, the source remains byte-identical, failure cases preserve existing destination files, 50-action history meets the memory budget, performance targets pass on the reference machine, and the package launches on a clean instance of the supported operating system.

## Success measures

In a five-person usability check, at least four users must independently open, crop, resize, and export a supplied image within 60 seconds without assistance. Collect timing manually; no production analytics are required.

The product is complete for v1 when this workflow is reliable, the release gates pass, and the installer or application package is available for the selected operating system.

## Technical references

- GPUI overview and upstream documentation: https://gpui.rs/
- GPUI image source API: https://docs.rs/gpui/latest/gpui/enum.ImageSource.html

These references establish framework capabilities. All feature scope, limits, architecture choices, and performance thresholds above are proposed product requirements.
