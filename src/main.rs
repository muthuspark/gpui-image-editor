mod import;

use gpui::{prelude::*, *};
use std::{path::PathBuf, sync::Arc};

actions!(image_editor, [OpenImage, Quit]);

struct Document {
    image: Arc<RenderImage>,
    filename: String,
    width: u32,
    height: u32,
}

struct ImageEditor {
    document: Option<Document>,
    busy: bool,
    error: Option<String>,
    focus: FocusHandle,
    import_focus: FocusHandle,
}

impl ImageEditor {
    fn pick_image(&mut self, _: &OpenImage, _: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import PNG or JPEG".into()),
        });
        self.busy = true;
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = selection.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(Ok(Some(paths))) => this.import_paths(&paths, cx),
                    Ok(Ok(None)) => {},
                    _ => this.error = Some("Could not open the file picker. Try dropping a PNG or JPEG onto the window.".into()),
                }
                cx.notify();
            });
        }).detach();
    }

    fn import_paths(&mut self, paths: &[PathBuf], cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if paths.len() != 1 {
            self.error = Some("Open one image at a time. Drop a single PNG or JPEG.".into());
            cx.notify();
            return;
        }
        let path = paths[0].clone();
        self.busy = true;
        self.error = None;
        cx.notify();
        let task = cx.background_executor().spawn(async move {
            import::open(&path).map(|imported| {
                let (width, height) = imported.pixels.dimensions();
                let mut pixels = imported.pixels;
                // GPUI uploads BGRA; decoding uses RGBA.
                for pixel in pixels.pixels_mut() {
                    pixel.0.swap(0, 2);
                }
                Document {
                    image: Arc::new(RenderImage::new(vec![image::Frame::new(pixels)])),
                    filename: imported.filename,
                    width,
                    height,
                }
            })
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(document) => this.document = Some(document),
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for ImageEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let preview = self.document.as_ref().map(|doc| {
            let scale = ((f32::from(viewport.width) - 80.) / doc.width as f32)
                .min((f32::from(viewport.height) - 180.) / doc.height as f32)
                .clamp(0.01, 1.);
            let width = px(doc.width as f32 * scale);
            let height = px(doc.height as f32 * scale);
            div()
                .relative()
                .w(width)
                .h(height)
                .bg(rgb(0x202024))
                .child(
                    canvas(
                        |_, _, _| (),
                        |bounds, _, window, _| {
                            let tile = 12.;
                            for row in 0..(f32::from(bounds.size.height) / tile).ceil() as u32 {
                                for col in 0..(f32::from(bounds.size.width) / tile).ceil() as u32 {
                                    if (row + col) % 2 == 0 {
                                        let x = col as f32 * tile;
                                        let y = row as f32 * tile;
                                        window.paint_quad(fill(
                                            Bounds::new(
                                                bounds.origin + point(px(x), px(y)),
                                                size(
                                                    px(tile.min(f32::from(bounds.size.width) - x)),
                                                    px(tile.min(f32::from(bounds.size.height) - y)),
                                                ),
                                            ),
                                            rgb(0x2a2a2e),
                                        ));
                                    }
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                .child(
                    img(doc.image.clone())
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                )
        });
        div()
            .id("image-editor")
            .key_context("ImageEditor")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, _| {
                if event.keystroke.key == "tab" {
                    window.focus(&this.import_focus);
                }
            }))
            .on_action(cx.listener(Self::pick_image))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.import_paths(paths.paths(), cx)
            }))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x09090b))
            .text_color(rgb(0xe4e4e7))
            .font_family("DejaVu Sans")
            .text_sm()
            .child(
                div()
                    .h(px(58.))
                    .flex_none()
                    .px_6()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(0x232326))
                    .child(div().text_color(rgb(0xa1a1aa)).child("Image Editor"))
                    .child(
                        div()
                            .id("import-image")
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x242427))
                            .border_1()
                            .border_color(rgb(0x39393e))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x343438)))
                            .track_focus(&self.import_focus)
                            .focus(|s| s.border_color(rgb(0xd4d4d8)))
                            .when(self.busy, |el| el.opacity(0.6))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.pick_image(&OpenImage, window, cx);
                                }
                            }))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.pick_image(&OpenImage, window, cx)
                            }))
                            .child(if self.busy {
                                "Opening…"
                            } else {
                                "Import image"
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .children(preview)
                    .when(self.document.is_none(), |el| {
                        el.child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_3()
                                .child(div().text_lg().child("Open an image or drop one here."))
                                .child(
                                    div()
                                        .text_color(rgb(0x96969f))
                                        .child("PNG or JPEG · Ctrl+O"),
                                ),
                        )
                    }),
            )
            .when_some(self.error.clone(), |el, error| {
                el.child(
                    div()
                        .px_6()
                        .py_3()
                        .bg(rgb(0x29191c))
                        .text_color(rgb(0xf5b4bd))
                        .child(error),
                )
            })
            .child(
                div()
                    .h(px(34.))
                    .flex_none()
                    .px_6()
                    .border_t_1()
                    .border_color(rgb(0x232326))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .text_xs()
                    .text_color(rgb(0x96969f))
                    .child(
                        div().flex_1().truncate().child(
                            self.document
                                .as_ref()
                                .map(|doc| doc.filename.clone())
                                .unwrap_or("No image open".into()),
                        ),
                    )
                    .child(div().flex_none().child(if self.busy {
                        "Opening image…".into()
                    } else {
                        self.document
                            .as_ref()
                            .map(|doc| format!("{} × {} px · Fit", doc.width, doc.height))
                            .unwrap_or("PNG / JPEG".into())
                    })),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("ctrl-o", OpenImage, Some("ImageEditor")),
            KeyBinding::new("ctrl-q", Quit, None),
        ]);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1100.), px(760.)),
                    cx,
                ))),
                window_min_size: Some(size(px(960.), px(640.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Image Editor".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus);
                    ImageEditor {
                        document: None,
                        busy: false,
                        error: None,
                        focus,
                        import_focus: cx.focus_handle(),
                    }
                })
            },
        )
        .expect("Could not open the Image Editor window");
        cx.activate(true);
    });
}
