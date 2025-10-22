pub mod build;
pub mod canvas;
pub mod node_kind;

use impact::egui::{
    Color32, CursorIcon, DragValue, FontId, Galley, Id, Painter, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use impact_dev_ui::option_panels::{LabelAndHoverText, option_drag_value};
use node_kind::MetaNodeKind;
use std::sync::Arc;
use tinyvec::TinyVec;

const NODE_CORNER_RADIUS: f32 = 8.0;
const NODE_FILL_COLOR: Color32 = Color32::from_gray(42);
const NODE_BOUNDARY_WIDTH: f32 = 1.0;
const SELECTED_NODE_BOUNDARY_WIDTH: f32 = 2.0;
const NODE_BOUNDARY_COLOR: Color32 = Color32::WHITE;
const SELECTED_NODE_BOUNDARY_COLOR: Color32 = Color32::YELLOW;

const NODE_TEXT_COLOR: Color32 = Color32::WHITE;
const NODE_HEADER_FONT_SIZE: f32 = 14.0;
const NODE_PARAMS_FONT_SIZE: f32 = 12.0;
const NODE_HEADER_SPACING: f32 = 8.0;
const NODE_PARAM_SPACING: f32 = 4.0;
const NODE_TEXT_PADDING: Vec2 = vec2(12.0, 12.0);

const PORT_RADIUS: f32 = 8.0;
const PORT_FILL_COLOR: Color32 = Color32::LIGHT_GRAY;
const HOVERED_PORT_FILL_COLOR: Color32 = Color32::WHITE;
const DISABLED_PORT_FILL_COLOR: Color32 = Color32::from_gray(80);

pub type MetaNodeID = u64;

#[derive(Clone, Debug)]
pub struct MetaNode {
    pub position: Pos2,
    pub data: MetaNodeData,
    pub parent: Option<MetaNodeID>,
    pub children: MetaNodeChildren,
}

type MetaNodeChildren = TinyVec<[Option<MetaNodeID>; 2]>;

#[derive(Clone, Debug)]
pub struct MetaNodeData {
    pub kind: MetaNodeKind,
    pub params: MetaNodeParams,
}

type MetaNodeParams = TinyVec<[MetaNodeParam; 12]>;

#[derive(Clone, Copy, Debug)]
pub struct MetaPortConfig {
    pub has_parent: bool,
    pub children: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaPort {
    Parent,
    Child { slot: usize, of: usize },
}

#[derive(Clone, Debug)]
pub enum MetaNodeParam {
    UInt(MetaUIntParam),
    Float(MetaFloatParam),
}

#[derive(Clone, Debug)]
pub struct MetaUIntParam {
    pub text: LabelAndHoverText,
    pub value: u32,
}

#[derive(Clone, Debug)]
pub struct MetaFloatParam {
    pub text: LabelAndHoverText,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub speed: f32,
}

impl MetaNode {
    fn new(position: Pos2, data: MetaNodeData) -> Self {
        let kind = data.kind;

        let mut children = MetaNodeChildren::new();
        children.resize(kind.port_config().children, None);

        Self {
            position,
            data,
            parent: None,
            children,
        }
    }

    fn change_kind(&mut self, new_kind: MetaNodeKind) {
        self.data.change_kind(new_kind);
        self.children.resize(new_kind.port_config().children, None);
    }

    fn first_free_child_slot(&self) -> Option<usize> {
        self.children.iter().position(|child| child.is_none())
    }

    fn get_node_attached_to_port(&self, port: MetaPort) -> Option<MetaNodeID> {
        match port {
            MetaPort::Parent => self.parent,
            MetaPort::Child { slot, .. } => self.children.get(slot).copied().flatten(),
        }
    }

    fn get_port_node_is_attached_to(
        &self,
        other_node_id: MetaNodeID,
        other_port: MetaPort,
    ) -> Option<MetaPort> {
        match other_port {
            MetaPort::Parent => self
                .children
                .iter()
                .position(|child| *child == Some(other_node_id))
                .map(|slot| MetaPort::Child {
                    slot,
                    of: self.children.len(),
                }),
            MetaPort::Child { .. } => {
                (self.parent == Some(other_node_id)).then_some(MetaPort::Parent)
            }
        }
    }
}

impl MetaNodeData {
    pub fn new(kind: MetaNodeKind) -> Self {
        Self {
            kind,
            params: kind.params(),
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

    fn prepare_header_text(&self, ui: &Ui, zoom: f32) -> Arc<Galley> {
        prepare_text(
            ui,
            self.kind.label().to_string(),
            Self::header_font(zoom),
            NODE_TEXT_COLOR,
        )
    }

    fn prepare_param_texts(&self, ui: &Ui, zoom: f32) -> impl Iterator<Item = Arc<Galley>> {
        self.params.iter().map(move |param| {
            prepare_text(
                ui,
                param.text_to_display(),
                Self::params_font(zoom),
                NODE_TEXT_COLOR,
            )
        })
    }

    fn compute_size(&self, ui: &Ui, zoom: f32) -> Vec2 {
        let header_text = self.prepare_header_text(ui, zoom);

        let mut max_text_width = header_text.size().x;
        let mut total_text_height = header_text.size().y;

        for param_text in self.prepare_param_texts(ui, zoom) {
            max_text_width = max_text_width.max(param_text.size().x);
            total_text_height += param_text.size().y;
        }

        if !self.params.is_empty() {
            total_text_height += Self::header_spacing(zoom)
                + Self::param_spacing(zoom) * ((self.params.len() - 1) as f32);
        }

        let screen_node_size =
            vec2(max_text_width, total_text_height) + 2.0 * Self::text_padding(zoom);

        screen_node_size / zoom
    }

    fn paint(&self, ui: &Ui, painter: &Painter, node_rect: Rect, zoom: f32, is_selected: bool) {
        // Draw node background and outline

        let (stroke_width, stroke_color) = if is_selected {
            (SELECTED_NODE_BOUNDARY_WIDTH, SELECTED_NODE_BOUNDARY_COLOR)
        } else {
            (NODE_BOUNDARY_WIDTH, NODE_BOUNDARY_COLOR)
        };
        let stroke = Stroke {
            width: stroke_width * zoom,
            color: stroke_color,
        };
        let corner_radius = NODE_CORNER_RADIUS * zoom;
        painter.rect_filled(node_rect, corner_radius, NODE_FILL_COLOR);
        painter.rect_stroke(node_rect, corner_radius, stroke, StrokeKind::Inside);

        // Draw node text
        self.paint_text(ui, painter, &node_rect, zoom);
    }

    fn paint_text(&self, ui: &Ui, painter: &Painter, node_rect: &Rect, zoom: f32) {
        let padding = Self::text_padding(zoom);

        let header_text = self.prepare_header_text(ui, zoom);
        let header_pos = pos2(
            node_rect.center().x - 0.5 * header_text.size().x,
            node_rect.top() + padding.y,
        );
        painter.galley(header_pos, header_text.clone(), NODE_TEXT_COLOR);

        let mut cursor = pos2(
            node_rect.left() + padding.x,
            header_pos.y + header_text.size().y + Self::header_spacing(zoom),
        );

        for param_text in self.prepare_param_texts(ui, zoom) {
            painter.galley(cursor, param_text.clone(), NODE_TEXT_COLOR);
            cursor.y += param_text.size().y + Self::param_spacing(zoom);
        }
    }

    fn change_kind(&mut self, new_kind: MetaNodeKind) {
        self.kind = new_kind;
        self.params = new_kind.params();
    }
}

impl MetaPortConfig {
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

    fn ports(&self) -> impl Iterator<Item = MetaPort> {
        self.has_parent
            .then_some(MetaPort::Parent)
            .into_iter()
            .chain((0..self.children).map(|slot| MetaPort::Child {
                slot,
                of: self.children,
            }))
    }
}

impl MetaPort {
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

    fn id(&self, node_id: MetaNodeID) -> Id {
        match self {
            Self::Parent => Id::new(("parent_port", node_id)),
            &Self::Child { slot, .. } => Id::new(("child_port", slot, node_id)),
        }
    }

    fn show(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        node_id: MetaNodeID,
        node_rect: &Rect,
        enabled: bool,
        highlighted: bool,
        zoom: f32,
        cursor_hidden: bool,
    ) -> Response {
        let center = self.center(node_rect);

        let port_radius = PORT_RADIUS * zoom;
        let hit_rect = Rect::from_center_size(center, vec2(2.0 * port_radius, 2.0 * port_radius));

        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(hit_rect, self.id(node_id), sense);

        if enabled && !cursor_hidden {
            response = response.on_hover_cursor(CursorIcon::PointingHand);
        }

        let color = if enabled {
            if highlighted || response.hovered() {
                HOVERED_PORT_FILL_COLOR
            } else {
                PORT_FILL_COLOR
            }
        } else {
            DISABLED_PORT_FILL_COLOR
        };

        painter.circle_filled(center, port_radius, color);

        response
    }
}

impl MetaNodeParam {
    pub fn show_controls(&mut self, ui: &mut Ui) -> Response {
        match self {
            Self::UInt(param) => param.show_controls(ui),
            Self::Float(param) => param.show_controls(ui),
        }
    }

    fn text_to_display(&self) -> String {
        match self {
            Self::UInt(param) => param.text_to_display(),
            Self::Float(param) => param.text_to_display(),
        }
    }

    fn as_uint(&self) -> Option<u32> {
        if let Self::UInt(param) = self {
            Some(param.value)
        } else {
            None
        }
    }

    fn uint(&self) -> u32 {
        self.as_uint().unwrap()
    }

    fn as_float(&self) -> Option<f32> {
        if let Self::Float(param) = self {
            Some(param.value)
        } else {
            None
        }
    }

    fn float(&self) -> f32 {
        self.as_float().unwrap()
    }
}

impl From<MetaUIntParam> for MetaNodeParam {
    fn from(param: MetaUIntParam) -> Self {
        Self::UInt(param)
    }
}

impl From<MetaFloatParam> for MetaNodeParam {
    fn from(param: MetaFloatParam) -> Self {
        Self::Float(param)
    }
}

impl Default for MetaNodeParam {
    fn default() -> Self {
        Self::UInt(MetaUIntParam {
            text: LabelAndHoverText::label_only(""),
            value: 0,
        })
    }
}

impl MetaUIntParam {
    const fn new(text: LabelAndHoverText, value: u32) -> Self {
        Self { text, value }
    }

    fn show_controls(&mut self, ui: &mut Ui) -> Response {
        option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value).speed(1),
        )
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text.label, self.value)
    }
}

impl MetaFloatParam {
    const fn new(text: LabelAndHoverText, value: f32) -> Self {
        Self {
            text,
            value,
            min_value: f32::NEG_INFINITY,
            max_value: f32::INFINITY,
            speed: 0.05,
        }
    }

    const fn with_min_value(mut self, min_value: f32) -> Self {
        self.min_value = min_value;
        self
    }

    const fn with_max_value(mut self, max_value: f32) -> Self {
        self.max_value = max_value;
        self
    }

    const fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    fn show_controls(&mut self, ui: &mut Ui) -> Response {
        option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value)
                .range(self.min_value..=self.max_value)
                .speed(self.speed),
        )
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text.label, self.value)
    }
}

fn prepare_text(ui: &Ui, text: String, font_id: FontId, color: Color32) -> Arc<Galley> {
    ui.fonts(|f| f.layout_no_wrap(text, font_id, color))
}
