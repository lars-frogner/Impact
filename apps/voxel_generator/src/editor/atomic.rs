pub mod build;
pub mod canvas;
pub mod node_kind;

use impact::egui::{
    Color32, FontId, Galley, Painter, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use impact_voxel::generation::sdf::SDFNodeID;
use node_kind::AtomicNodeKind;
use std::sync::Arc;
use tinyvec::TinyVec;

const NODE_CORNER_RADIUS: f32 = 8.0;
const NODE_FILL_COLOR: Color32 = Color32::from_gray(42);
const NODE_BOUNDARY_WIDTH: f32 = 1.0;
const NODE_BOUNDARY_COLOR: Color32 = Color32::WHITE;

const NODE_TEXT_COLOR: Color32 = Color32::WHITE;
const NODE_HEADER_FONT_SIZE: f32 = 14.0;
const NODE_PARAMS_FONT_SIZE: f32 = 12.0;
const NODE_HEADER_SPACING: f32 = 8.0;
const NODE_PARAM_SPACING: f32 = 4.0;
const NODE_TEXT_PADDING: Vec2 = vec2(12.0, 12.0);

const PORT_RADIUS: f32 = 0.0; // Hide ports
const PORT_FILL_COLOR: Color32 = Color32::from_gray(80);

#[derive(Clone, Debug)]
pub struct AtomicNode {
    pub position: Pos2,
    pub data: AtomicNodeData,
    pub parents: AtomicNodeParents,
    pub children: AtomicNodeChildren,
}

type AtomicNodeParents = TinyVec<[SDFNodeID; 8]>;
type AtomicNodeChildren = TinyVec<[SDFNodeID; 2]>;

#[derive(Clone, Debug)]
pub struct AtomicNodeData {
    pub kind: AtomicNodeKind,
    pub params: AtomicNodeParams,
    prepared_text_zoom: Option<f32>,
    header_galley: Option<Arc<Galley>>,
    param_galleys: TinyVec<[Option<Arc<Galley>>; 12]>,
}

type AtomicNodeParams = TinyVec<[AtomicNodeParam; 12]>;

#[derive(Clone, Copy, Debug)]
pub struct AtomicPortConfig {
    pub has_parent: bool,
    pub children: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicPort {
    Parent,
    Child { slot: usize, of: usize },
}

#[derive(Clone, Debug)]
pub enum AtomicNodeParam {
    UInt(AtomicUIntParam),
    Float(AtomicFloatParam),
}

#[derive(Clone, Debug)]
pub struct AtomicUIntParam {
    pub text: &'static str,
    pub value: u32,
}

#[derive(Clone, Debug)]
pub struct AtomicFloatParam {
    pub text: &'static str,
    pub value: f32,
}

impl AtomicNode {
    fn new_output(child_id: SDFNodeID) -> Self {
        let mut children = AtomicNodeChildren::new();
        children.push(child_id);
        Self {
            data: AtomicNodeData::new(AtomicNodeKind::Output, AtomicNodeParams::new()),
            children,
            parents: AtomicNodeParents::new(),
            position: Pos2::ZERO,
        }
    }

    fn new_leaf(kind: AtomicNodeKind, params: AtomicNodeParams) -> Self {
        Self {
            data: AtomicNodeData::new(kind, params),
            children: AtomicNodeChildren::new(),
            // Parents and position are assigned later
            parents: AtomicNodeParents::new(),
            position: Pos2::ZERO,
        }
    }

    fn new_unary(kind: AtomicNodeKind, params: AtomicNodeParams, child_id: SDFNodeID) -> Self {
        let mut children = AtomicNodeChildren::new();
        children.push(child_id);
        Self {
            data: AtomicNodeData::new(kind, params),
            children,
            // Parents and position are assigned later
            parents: AtomicNodeParents::new(),
            position: Pos2::ZERO,
        }
    }

    fn new_binary(
        kind: AtomicNodeKind,
        params: AtomicNodeParams,
        child_1_id: SDFNodeID,
        child_2_id: SDFNodeID,
    ) -> Self {
        let mut children = AtomicNodeChildren::new();
        children.push(child_1_id);
        children.push(child_2_id);
        Self {
            data: AtomicNodeData::new(kind, params),
            children,
            // Parents and position are assigned later
            parents: AtomicNodeParents::new(),
            position: Pos2::ZERO,
        }
    }
}

impl AtomicNodeData {
    fn new(kind: AtomicNodeKind, params: AtomicNodeParams) -> Self {
        Self {
            kind,
            params,
            prepared_text_zoom: None,
            header_galley: None,
            param_galleys: TinyVec::new(),
        }
    }

    fn header_font(zoom: f32) -> FontId {
        FontId::proportional(NODE_HEADER_FONT_SIZE * zoom)
    }

    fn params_font(zoom: f32) -> FontId {
        FontId::proportional(NODE_PARAMS_FONT_SIZE * zoom)
    }

    fn text_padding(zoom: f32) -> Vec2 {
        NODE_TEXT_PADDING * zoom
    }

    fn header_spacing(zoom: f32) -> f32 {
        NODE_HEADER_SPACING * zoom
    }

    fn param_spacing(zoom: f32) -> f32 {
        NODE_PARAM_SPACING * zoom
    }

    fn prepare_text(&mut self, ui: &Ui, zoom: f32) {
        if self
            .prepared_text_zoom
            .is_some_and(|prepared_text_zoom| prepared_text_zoom == zoom)
        {
            return;
        }
        self.prepared_text_zoom = Some(zoom);

        self.header_galley = Some(prepare_text(
            ui,
            self.kind.label().to_string(),
            Self::header_font(zoom),
            NODE_TEXT_COLOR,
        ));

        self.param_galleys.clear();
        self.param_galleys
            .extend(self.params.iter().map(move |param| {
                Some(prepare_text(
                    ui,
                    param.text_to_display(),
                    Self::params_font(zoom),
                    NODE_TEXT_COLOR,
                ))
            }));
    }

    fn prepared_header_text(&self) -> &Arc<Galley> {
        self.header_galley.as_ref().unwrap()
    }

    fn prepared_param_texts(&self) -> impl Iterator<Item = &Arc<Galley>> {
        self.param_galleys.iter().map(|g| g.as_ref().unwrap())
    }

    fn compute_size(&self) -> Vec2 {
        let unzooming_factor = self.prepared_text_zoom.unwrap().recip();

        let header_text_size = self.prepared_header_text().size() * unzooming_factor;

        let mut max_text_width = header_text_size.x;
        let mut total_text_height = header_text_size.y;

        for param_text in self.prepared_param_texts() {
            let param_text_size = param_text.size() * unzooming_factor;
            max_text_width = max_text_width.max(param_text_size.x);
            total_text_height += param_text_size.y;
        }

        if !self.params.is_empty() {
            total_text_height += Self::header_spacing(1.0)
                + Self::param_spacing(1.0) * ((self.params.len() - 1) as f32);
        }

        let node_size = vec2(max_text_width, total_text_height) + 2.0 * Self::text_padding(1.0);

        node_size
    }

    fn paint(&self, painter: &Painter, node_rect: Rect, zoom: f32) {
        // Draw node background and outline

        let stroke = Stroke {
            width: NODE_BOUNDARY_WIDTH * zoom,
            color: NODE_BOUNDARY_COLOR,
        };
        let corner_radius = NODE_CORNER_RADIUS * zoom;
        painter.rect_filled(node_rect, corner_radius, NODE_FILL_COLOR);
        painter.rect_stroke(node_rect, corner_radius, stroke, StrokeKind::Inside);

        // Draw node text
        self.paint_text(painter, &node_rect, zoom);
    }

    fn paint_text(&self, painter: &Painter, node_rect: &Rect, zoom: f32) {
        let padding = Self::text_padding(zoom);

        let header_text = self.prepared_header_text();
        let header_pos = pos2(
            node_rect.center().x - 0.5 * header_text.size().x,
            node_rect.top() + padding.y,
        );
        painter.galley(header_pos, header_text.clone(), NODE_TEXT_COLOR);

        let mut cursor = pos2(
            node_rect.left() + padding.x,
            header_pos.y + header_text.size().y + Self::header_spacing(zoom),
        );

        for param_text in self.prepared_param_texts() {
            painter.galley(cursor, param_text.clone(), NODE_TEXT_COLOR);
            cursor.y += param_text.size().y + Self::param_spacing(zoom);
        }
    }
}

impl AtomicPortConfig {
    const fn root() -> Self {
        Self {
            has_parent: false,
            children: 1,
        }
    }

    const fn leaf() -> Self {
        Self {
            has_parent: true,
            children: 0,
        }
    }

    const fn unary() -> Self {
        Self {
            has_parent: true,
            children: 1,
        }
    }

    const fn binary() -> Self {
        Self {
            has_parent: true,
            children: 2,
        }
    }

    fn ports(&self) -> impl Iterator<Item = AtomicPort> {
        self.has_parent
            .then_some(AtomicPort::Parent)
            .into_iter()
            .chain((0..self.children).map(|slot| AtomicPort::Child {
                slot,
                of: self.children,
            }))
    }
}

impl AtomicPort {
    fn center(&self, node_rect: &Rect) -> Pos2 {
        match self {
            Self::Parent => node_rect.center_top(),
            &Self::Child { slot, of } => {
                node_rect.left_bottom()
                    + vec2(
                        (1.0 + slot as f32) * node_rect.width() / (of as f32 + 1.0),
                        0.0,
                    )
            }
        }
    }

    fn paint(&self, painter: &Painter, node_rect: &Rect, zoom: f32) {
        let center = self.center(node_rect);
        let port_radius = PORT_RADIUS * zoom;
        painter.circle_filled(center, port_radius, PORT_FILL_COLOR);
    }
}

impl AtomicNodeParam {
    fn text_to_display(&self) -> String {
        match self {
            Self::UInt(param) => param.text_to_display(),
            Self::Float(param) => param.text_to_display(),
        }
    }
}

impl From<AtomicUIntParam> for AtomicNodeParam {
    fn from(param: AtomicUIntParam) -> Self {
        Self::UInt(param)
    }
}

impl From<AtomicFloatParam> for AtomicNodeParam {
    fn from(param: AtomicFloatParam) -> Self {
        Self::Float(param)
    }
}

impl Default for AtomicNodeParam {
    fn default() -> Self {
        Self::UInt(AtomicUIntParam { text: "", value: 0 })
    }
}

impl AtomicUIntParam {
    const fn new(text: &'static str, value: u32) -> Self {
        Self { text, value }
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text, self.value)
    }
}

impl AtomicFloatParam {
    const fn new(text: &'static str, value: f32) -> Self {
        Self { text, value }
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text, self.value)
    }
}

fn prepare_text(ui: &Ui, text: String, font_id: FontId, color: Color32) -> Arc<Galley> {
    ui.fonts(|f| f.layout_no_wrap(text, font_id, color))
}
