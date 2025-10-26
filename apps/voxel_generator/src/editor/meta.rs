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

const PORT_RADIUS: f32 = 6.0;
const PORT_HOVER_RADIUS: f32 = 7.0;
const PORT_HIT_RADIUS: f32 = 14.0;
const PORT_FILL_COLOR: Color32 = Color32::LIGHT_GRAY;
const HOVERED_PORT_FILL_COLOR: Color32 = Color32::WHITE;
const DISABLED_PORT_FILL_COLOR: Color32 = Color32::from_gray(80);

pub type MetaNodeID = u64;

#[derive(Clone, Debug)]
pub struct MetaNode {
    pub position: Pos2,
    pub data: MetaNodeData,
    pub links_to_parents: MetaNodeParentLinks,
    pub links_to_children: MetaNodeChildLinks,
}

type MetaNodeParentLinks = TinyVec<[Option<MetaNodeLink>; 2]>;
type MetaNodeChildLinks = TinyVec<[Option<MetaNodeLink>; 2]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetaNodeLink {
    pub to_node: MetaNodeID,
    pub to_slot: usize,
}

#[derive(Clone, Debug)]
pub struct MetaNodeData {
    pub kind: MetaNodeKind,
    pub params: MetaNodeParams,
    prepared_text_zoom: Option<f32>,
    header_galley: Option<Arc<Galley>>,
    param_galleys: TinyVec<[Option<Arc<Galley>>; 12]>,
}

type MetaNodeParams = TinyVec<[MetaNodeParam; 12]>;

#[derive(Clone, Copy, Debug)]
pub struct MetaPortConfig {
    pub children: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaPort {
    Parent { slot: usize, of: usize },
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

        let mut parent_links = MetaNodeParentLinks::new();
        if !kind.is_root() {
            parent_links.resize(1, None);
        }

        let mut child_links = MetaNodeChildLinks::new();
        child_links.resize(kind.port_config().children, None);

        Self {
            position,
            data,
            links_to_parents: parent_links,
            links_to_children: child_links,
        }
    }

    fn change_kind(&mut self, new_kind: MetaNodeKind) {
        self.data.change_kind(new_kind);
        self.links_to_children
            .resize(new_kind.port_config().children, None);
    }

    fn first_free_child_slot(&self) -> Option<usize> {
        self.links_to_children
            .iter()
            .position(|link| link.is_none())
    }
}

impl MetaNodeData {
    pub fn new(kind: MetaNodeKind) -> Self {
        Self {
            kind,
            params: kind.params(),
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

    /// Returns `true` if any of the parameters changed.
    pub fn run_controls(&mut self, ui: &mut Ui) -> bool {
        let mut any_param_changed = false;
        for (idx, param) in self.params.iter_mut().enumerate() {
            if param.show_controls(ui).changed() {
                any_param_changed = true;

                if let (Some(zoom), Some(gally)) =
                    (self.prepared_text_zoom, self.param_galleys.get_mut(idx))
                {
                    gally.replace(prepare_text(
                        ui,
                        param.text_to_display(),
                        Self::params_font(zoom),
                        NODE_TEXT_COLOR,
                    ));
                }
            };
        }
        any_param_changed
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

        let screen_node_size =
            vec2(max_text_width, total_text_height) + 2.0 * Self::text_padding(1.0);

        screen_node_size
    }

    fn paint(&self, painter: &Painter, node_rect: Rect, zoom: f32, is_selected: bool) {
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

    fn change_kind(&mut self, new_kind: MetaNodeKind) {
        self.kind = new_kind;
        self.params = new_kind.params();
        self.prepared_text_zoom = None;
    }
}

impl MetaPortConfig {
    const fn root() -> Self {
        Self { children: 1 }
    }

    const fn leaf() -> Self {
        Self { children: 0 }
    }

    const fn unary() -> Self {
        Self { children: 1 }
    }

    const fn binary() -> Self {
        Self { children: 2 }
    }

    fn ports(&self, parents: usize) -> impl Iterator<Item = MetaPort> {
        (0..parents)
            .map(move |slot| MetaPort::Parent { slot, of: parents })
            .chain((0..self.children).map(|slot| MetaPort::Child {
                slot,
                of: self.children,
            }))
    }
}

impl MetaPort {
    fn center(&self, node_rect: &Rect) -> Pos2 {
        match *self {
            Self::Parent { slot, of } => {
                node_rect.left_top()
                    + vec2(
                        (1.0 + slot as f32) * node_rect.width() / (of as f32 + 1.0),
                        0.0,
                    )
            }
            Self::Child { slot, of } => {
                node_rect.left_bottom()
                    + vec2(
                        (1.0 + slot as f32) * node_rect.width() / (of as f32 + 1.0),
                        0.0,
                    )
            }
        }
    }

    fn id(&self, node_id: MetaNodeID) -> Id {
        match *self {
            Self::Parent { slot, .. } => Id::new(("parent_port", slot, node_id)),
            Self::Child { slot, .. } => Id::new(("child_port", slot, node_id)),
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

        let port_hit_diameter = 2.0 * PORT_HIT_RADIUS * zoom;
        let hit_rect = Rect::from_center_size(center, vec2(port_hit_diameter, port_hit_diameter));

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

        let port_radius = if enabled && response.hovered() {
            PORT_HOVER_RADIUS * zoom
        } else {
            PORT_RADIUS * zoom
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
