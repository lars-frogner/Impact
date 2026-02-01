//! General-purpose text overlay helper.

use impact::egui::{Align2, Area, Context, Id, Pos2, TextStyle, Vec2, vec2};
use tinyvec::TinyVec;

/// Corner anchor for overlay positioning.
#[derive(Clone, Copy, Debug, Default)]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Size specification for the overlay.
#[derive(Clone, Copy, Debug, Default)]
pub enum OverlaySize {
    /// Size adapts to fit content.
    #[default]
    Adaptive,
    /// Fixed width, adaptive height.
    FixedWidth(f32),
    /// Fixed dimensions.
    Fixed(Vec2),
}

/// Builder for creating text overlays anchored to screen corners.
#[derive(Debug)]
pub struct TextOverlay {
    id: Id,
    corner: Corner,
    offset: Vec2,
    size: OverlaySize,
    text_style: TextStyle,
}

impl TextOverlay {
    /// Creates a new text overlay with the given ID.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            corner: Corner::default(),
            offset: vec2(10.0, 10.0),
            size: OverlaySize::default(),
            text_style: TextStyle::Body,
        }
    }

    /// Sets the corner to anchor the overlay to.
    pub fn corner(mut self, corner: Corner) -> Self {
        self.corner = corner;
        self
    }

    /// Sets the offset from the corner (always specified as positive values).
    pub fn offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the size behavior of the overlay.
    pub fn size(mut self, size: OverlaySize) -> Self {
        self.size = size;
        self
    }

    /// Sets the text style to use.
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = style;
        self
    }

    /// Shows the overlay with a single line of text.
    pub fn show(self, ctx: &Context, text: &str) {
        self.show_lines(ctx, &[text]);
    }

    /// Shows the overlay with multiple lines of text.
    pub fn show_lines(self, ctx: &Context, lines: &[&str]) {
        let (anchor, sign) = match self.corner {
            Corner::TopLeft => (Align2::LEFT_TOP, vec2(1.0, 1.0)),
            Corner::TopRight => (Align2::RIGHT_TOP, vec2(-1.0, 1.0)),
            Corner::BottomLeft => (Align2::LEFT_BOTTOM, vec2(1.0, -1.0)),
            Corner::BottomRight => (Align2::RIGHT_BOTTOM, vec2(-1.0, -1.0)),
        };

        let offset = vec2(self.offset.x * sign.x, self.offset.y * sign.y);

        Area::new(self.id)
            .anchor(anchor, offset)
            .interactable(false)
            .show(ctx, |ui| {
                let font_id = self.text_style.resolve(ui.style());
                let color = ui.visuals().text_color();

                let text_align = match self.corner {
                    Corner::TopLeft | Corner::BottomLeft => Align2::LEFT_TOP,
                    Corner::TopRight | Corner::BottomRight => Align2::RIGHT_TOP,
                };

                let galleys: TinyVec<[_; 16]> = ctx.fonts_mut(|f| {
                    lines
                        .iter()
                        .map(|line| {
                            Some(f.layout_no_wrap(
                                line.to_string(),
                                font_id.clone(),
                                Default::default(),
                            ))
                        })
                        .collect()
                });

                let content_width = galleys
                    .iter()
                    .map(|g| g.as_ref().unwrap().rect.width())
                    .fold(0.0f32, f32::max);

                let content_height: f32 = galleys
                    .iter()
                    .map(|g| g.as_ref().unwrap().rect.height())
                    .sum();

                let size = match self.size {
                    OverlaySize::Adaptive => vec2(content_width, content_height),
                    OverlaySize::FixedWidth(w) => vec2(w, content_height),
                    OverlaySize::Fixed(s) => s,
                };

                let rect = ui.max_rect();
                let (start_x, mut y) = match self.corner {
                    Corner::TopLeft => (rect.left(), rect.top()),
                    Corner::TopRight => (rect.right(), rect.top()),
                    Corner::BottomLeft => (rect.left(), rect.bottom() - content_height),
                    Corner::BottomRight => (rect.right(), rect.bottom() - content_height),
                };

                for (line, galley) in lines.iter().zip(&galleys) {
                    ui.painter().text(
                        Pos2::new(start_x, y),
                        text_align,
                        *line,
                        font_id.clone(),
                        color,
                    );
                    y += galley.as_ref().unwrap().rect.height();
                }

                ui.allocate_space(size);
            });
    }
}
