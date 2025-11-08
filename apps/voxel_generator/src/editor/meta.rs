pub mod build;
pub mod canvas;
pub mod data_type;
pub mod io;
pub mod node_kind;

use data_type::{EdgeDataType, input_and_output_types_for_new_node};
use impact::egui::{
    Color32, CursorIcon, DragValue, FontId, Galley, Id, Painter, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, Vec2, emath::Numeric, pos2, vec2,
};
use impact_dev_ui::option_panels::{LabelAndHoverText, labeled_option, option_drag_value};
use impact_voxel::generation::sdf::meta::{ContParamRange, DiscreteParamRange};
use node_kind::MetaNodeKind;
use node_kind::{MetaChildPortKind, MetaParentPortKind};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tinyvec::TinyVec;

const NODE_CORNER_RADIUS: f32 = 8.0;
const NODE_FILL_COLOR: Color32 = Color32::from_gray(42);
const NODE_BOUNDARY_WIDTH: f32 = 1.0;
const SELECTED_NODE_BOUNDARY_WIDTH: f32 = 2.0;
const NODE_BOUNDARY_COLOR: Color32 = Color32::from_gray(200);
const SELECTED_NODE_BOUNDARY_COLOR: Color32 = Color32::WHITE;
const NODE_STACK_SILHOUETTE_OFFSET: f32 = 5.0;
const NODE_STACK_SILHOUETTE_BOUNDARY_COLOR: Color32 = Color32::from_gray(100);
const SELECTED_NODE_STACK_SILHOUETTE_BOUNDARY_COLOR: Color32 = Color32::from_gray(180);

const NODE_TEXT_COLOR: Color32 = Color32::WHITE;
const NODE_HEADER_FONT_SIZE: f32 = 14.0;
const NODE_PARAMS_FONT_SIZE: f32 = 12.0;
const NODE_NAME_FONT_SIZE: f32 = 16.0;
const NODE_HEADER_SPACING: f32 = 8.0;
const NODE_PARAM_SPACING: f32 = 4.0;
const NODE_STANDARD_TEXT_PADDING: Vec2 = vec2(12.0, 12.0);
const NODE_COLLAPSED_TEXT_PADDING: Vec2 = vec2(14.0, 14.0);

const PORT_RADIUS: f32 = 6.0;
const PORT_HOVER_RADIUS: f32 = 7.0;
const PORT_HIT_RADIUS: f32 = 14.0;

pub type MetaNodeID = u64;

#[derive(Clone, Debug)]
pub struct MetaNode {
    pub position: Pos2,
    pub data: MetaNodeData,
    pub links_to_parents: MetaNodeParentLinks,
    pub links_to_children: MetaNodeChildLinks,
    pub output_data_type: EdgeDataType,
    pub input_data_types: MetaNodeInputDataTypes,
}

type MetaNodeParentLinks = TinyVec<[Option<MetaNodeLink>; 2]>;
type MetaNodeChildLinks = TinyVec<[Option<MetaNodeLink>; 2]>;
type MetaNodeInputDataTypes = TinyVec<[EdgeDataType; 2]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetaNodeLink {
    pub to_node: MetaNodeID,
    pub to_slot: usize,
}

#[derive(Clone, Debug)]
pub struct MetaNodeData {
    pub name: String,
    pub kind: MetaNodeKind,
    pub params: MetaNodeParams,
    prepared_text_zoom: Option<f32>,
    header_galley: Option<Arc<Galley>>,
    param_galleys: TinyVec<[Option<Arc<Galley>>; 12]>,
    name_galley: Option<Arc<Galley>>,
}

type MetaNodeParams = TinyVec<[MetaNodeParam; 12]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaPort {
    Parent {
        kind: MetaParentPortKind,
        slot: usize,
    },
    Child {
        kind: MetaChildPortKind,
        slot: usize,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MetaPaletteColor {
    standard: Color32,
    lighter: Color32,
    darker: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetaPortShape {
    Circle,
    Square,
}

#[derive(Clone, Debug)]
pub enum MetaNodeParam {
    UInt(MetaUIntParam),
    Float(MetaFloatParam),
    UIntRange(MetaUIntRangeParam),
    FloatRange(MetaFloatRangeParam),
}

#[derive(Clone, Debug)]
pub struct MetaUIntParam {
    pub text: LabelAndHoverText,
    pub value: u32,
    pub speed: f32,
}

#[derive(Clone, Debug)]
pub struct MetaFloatParam {
    pub text: LabelAndHoverText,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub speed: f32,
}

#[derive(Clone, Debug)]
pub struct MetaUIntRangeParam {
    pub text: LabelAndHoverText,
    pub low_value: u32,
    pub high_value: u32,
    pub speed: f32,
}

#[derive(Clone, Debug)]
pub struct MetaFloatRangeParam {
    pub text: LabelAndHoverText,
    pub low_value: f32,
    pub high_value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub speed: f32,
}

#[derive(Clone, Debug, Default)]
pub struct CollapsedMetaSubtree {
    pub size: Vec2,
    pub exposed_parent_ports: CollapsedMetaSubtreeParentPorts,
    pub exposed_child_ports: CollapsedMetaSubtreeChildPorts,
}

type CollapsedMetaSubtreeParentPorts = TinyVec<[CollapsedMetaSubtreeParentPort; 4]>;
type CollapsedMetaSubtreeChildPorts = TinyVec<[CollapsedMetaSubtreeChildPort; 4]>;

#[derive(Clone, Debug, Default)]
pub struct CollapsedMetaSubtreeParentPort {
    on_node: MetaNodeID,
    slot_on_node: usize,
    kind: MetaParentPortKind,
    output_data_type: EdgeDataType,
    link: Option<MetaNodeLink>,
}

#[derive(Clone, Debug, Default)]
pub struct CollapsedMetaSubtreeChildPort {
    on_node: MetaNodeID,
    slot_on_node: usize,
    kind: MetaChildPortKind,
    input_data_type: EdgeDataType,
    link: Option<MetaNodeLink>,
}

impl MetaNode {
    fn new(position: Pos2, data: MetaNodeData) -> Self {
        let kind = data.kind;

        let mut links_to_parents = MetaNodeParentLinks::new();
        if !kind.is_output() {
            links_to_parents.resize(1, None);
        }

        let mut links_to_children = MetaNodeChildLinks::new();
        links_to_children.resize(kind.n_child_slots(), None);

        Self::new_with_links(position, data, links_to_parents, links_to_children)
    }

    fn new_with_links(
        position: Pos2,
        data: MetaNodeData,
        links_to_parents: MetaNodeParentLinks,
        links_to_children: MetaNodeChildLinks,
    ) -> Self {
        let kind = data.kind;

        let (input_data_types, output_data_type) = input_and_output_types_for_new_node(kind);

        Self {
            position,
            data,
            links_to_parents,
            links_to_children,
            output_data_type,
            input_data_types,
        }
    }

    fn port_position(&self, node_rect: &Rect, port: MetaPort) -> Pos2 {
        match port {
            MetaPort::Parent { slot, .. } => {
                MetaPort::parent_center(node_rect, slot, self.links_to_parents.len())
            }
            MetaPort::Child { slot, .. } => {
                MetaPort::child_center(node_rect, slot, self.links_to_children.len())
            }
        }
    }

    fn change_kind(&mut self, new_kind: MetaNodeKind) {
        self.data.change_kind(new_kind);
        self.links_to_children
            .resize(new_kind.n_child_slots(), None);
    }

    fn first_free_child_slot_accepting_type(
        &self,
        output_data_type: EdgeDataType,
    ) -> Option<usize> {
        self.links_to_children
            .iter()
            .zip(&self.input_data_types)
            .position(|(link, input_data_type)| {
                link.is_none()
                    && EdgeDataType::connection_allowed(*input_data_type, output_data_type)
            })
    }
}

impl MetaNodeData {
    pub fn new(name: String, kind: MetaNodeKind, params: MetaNodeParams) -> Self {
        Self {
            name,
            kind,
            params,
            prepared_text_zoom: None,
            header_galley: None,
            param_galleys: TinyVec::new(),
            name_galley: None,
        }
    }

    pub fn new_default(kind: MetaNodeKind) -> Self {
        Self::new(String::new(), kind, kind.params())
    }

    fn header_font(zoom: f32) -> FontId {
        FontId::proportional(NODE_HEADER_FONT_SIZE * zoom)
    }

    fn params_font(zoom: f32) -> FontId {
        FontId::proportional(NODE_PARAMS_FONT_SIZE * zoom)
    }

    fn name_font(zoom: f32) -> FontId {
        FontId::proportional(NODE_NAME_FONT_SIZE * zoom)
    }

    fn standard_text_padding(zoom: f32) -> Vec2 {
        NODE_STANDARD_TEXT_PADDING * zoom
    }

    fn collapsed_text_padding(zoom: f32) -> Vec2 {
        NODE_COLLAPSED_TEXT_PADDING * zoom
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

    pub fn prepare_text(&mut self, ui: &Ui, zoom: f32) {
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

        self.name_galley = Some(prepare_text(
            ui,
            self.name.clone(),
            Self::name_font(zoom),
            NODE_TEXT_COLOR,
        ));
    }

    pub fn reprepare_name_text(&mut self, ui: &Ui) {
        if let Some(zoom) = self.prepared_text_zoom {
            self.name_galley.replace(prepare_text(
                ui,
                self.name.clone(),
                Self::name_font(zoom),
                NODE_TEXT_COLOR,
            ));
        }
    }

    fn prepared_header_text(&self) -> &Arc<Galley> {
        self.header_galley.as_ref().unwrap()
    }

    fn prepared_param_texts(&self) -> impl Iterator<Item = &Arc<Galley>> {
        self.param_galleys.iter().map(|g| g.as_ref().unwrap())
    }

    fn prepared_name_text(&self) -> &Arc<Galley> {
        self.name_galley.as_ref().unwrap()
    }

    fn compute_standard_size(&self) -> Vec2 {
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
            vec2(max_text_width, total_text_height) + 2.0 * Self::standard_text_padding(1.0);

        screen_node_size
    }

    fn compute_collapsed_size(&self) -> Vec2 {
        let unzooming_factor = self.prepared_text_zoom.unwrap().recip();

        let name_text_size = self.prepared_name_text().size() * unzooming_factor;

        let screen_node_size = name_text_size + 2.0 * Self::collapsed_text_padding(1.0);

        screen_node_size
    }

    fn paint(
        &self,
        painter: &Painter,
        node_rect: Rect,
        zoom: f32,
        is_selected: bool,
        is_collapsed: bool,
    ) {
        // Draw node background and outline

        let corner_radius = NODE_CORNER_RADIUS * zoom;

        let (stroke_width, stroke_color) = if is_selected {
            (SELECTED_NODE_BOUNDARY_WIDTH, SELECTED_NODE_BOUNDARY_COLOR)
        } else {
            (NODE_BOUNDARY_WIDTH, NODE_BOUNDARY_COLOR)
        };
        let stroke = Stroke {
            width: stroke_width * zoom,
            color: stroke_color,
        };

        if is_collapsed {
            // Stacked-card silhouette
            let back_offset = -NODE_STACK_SILHOUETTE_OFFSET * zoom;
            let back_rect = node_rect.translate(Vec2::splat(back_offset));

            let back_stroke_color = if is_selected {
                SELECTED_NODE_STACK_SILHOUETTE_BOUNDARY_COLOR
            } else {
                NODE_STACK_SILHOUETTE_BOUNDARY_COLOR
            };
            let back_stroke = Stroke {
                width: stroke.width,
                color: back_stroke_color,
            };
            painter.rect_filled(back_rect, corner_radius, NODE_FILL_COLOR);
            painter.rect_stroke(back_rect, corner_radius, back_stroke, StrokeKind::Inside);
        }

        painter.rect_filled(node_rect, corner_radius, NODE_FILL_COLOR);
        painter.rect_stroke(node_rect, corner_radius, stroke, StrokeKind::Inside);

        // Draw node text
        if is_collapsed {
            self.paint_collapsed_text(painter, &node_rect, zoom);
        } else {
            self.paint_standard_text(painter, &node_rect, zoom);
        }
    }

    fn paint_standard_text(&self, painter: &Painter, node_rect: &Rect, zoom: f32) {
        let padding = Self::standard_text_padding(zoom);

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

    fn paint_collapsed_text(&self, painter: &Painter, node_rect: &Rect, zoom: f32) {
        let padding = Self::collapsed_text_padding(zoom);

        let name_text = self.prepared_name_text();
        let name_pos = pos2(
            node_rect.center().x - 0.5 * name_text.size().x,
            node_rect.top() + padding.y,
        );
        painter.galley(name_pos, name_text.clone(), NODE_TEXT_COLOR);
    }

    fn change_kind(&mut self, new_kind: MetaNodeKind) {
        self.kind = new_kind;
        self.params = new_kind.params();
        self.prepared_text_zoom = None;
    }
}

impl MetaPort {
    fn parent_center(node_rect: &Rect, slot: usize, of: usize) -> Pos2 {
        node_rect.left_top() + Self::center_offset(node_rect, slot, of)
    }

    fn child_center(node_rect: &Rect, slot: usize, of: usize) -> Pos2 {
        node_rect.left_bottom() + Self::center_offset(node_rect, slot, of)
    }

    fn center_offset(node_rect: &Rect, slot: usize, of: usize) -> Vec2 {
        vec2(
            (1.0 + slot as f32) * node_rect.width() / (of as f32 + 1.0),
            0.0,
        )
    }

    fn id(&self, node_id: MetaNodeID) -> Id {
        match *self {
            Self::Parent { slot, .. } => Id::new(("parent_port", slot, node_id)),
            Self::Child { slot, .. } => Id::new(("child_port", slot, node_id)),
        }
    }
}

impl MetaPaletteColor {
    pub const fn red() -> Self {
        Self {
            standard: Color32::from_rgb(255, 60, 56), // #FF3C38
            lighter: Color32::from_rgb(255, 74, 71),  // #FF4A47
            darker: Color32::from_rgb(163, 3, 0),     // #A30300
        }
    }

    pub const fn green() -> Self {
        Self {
            standard: Color32::from_rgb(59, 193, 74), // #3BC14A
            lighter: Color32::from_rgb(84, 201, 96),  // #54C960
            darker: Color32::from_rgb(34, 109, 42),   // #226D2A
        }
    }

    pub const fn blue() -> Self {
        Self {
            standard: Color32::from_rgb(58, 128, 207), // #3A80CF
            lighter: Color32::from_rgb(124, 170, 223), // #7CAADF
            darker: Color32::from_rgb(28, 68, 115),    // #1C4473
        }
    }

    pub const fn yellow() -> Self {
        Self {
            standard: Color32::from_rgb(255, 214, 51), // #FFD633
            lighter: Color32::from_rgb(255, 231, 133), // #FFE785
            darker: Color32::from_rgb(122, 98, 0),     // #7A6200
        }
    }
}

impl MetaNodeParam {
    pub fn show_controls(&mut self, ui: &mut Ui) -> Response {
        match self {
            Self::UInt(param) => param.show_controls(ui),
            Self::Float(param) => param.show_controls(ui),
            Self::UIntRange(param) => param.show_controls(ui),
            Self::FloatRange(param) => param.show_controls(ui),
        }
    }

    fn text_to_display(&self) -> String {
        match self {
            Self::UInt(param) => param.text_to_display(),
            Self::Float(param) => param.text_to_display(),
            Self::UIntRange(param) => param.text_to_display(),
            Self::FloatRange(param) => param.text_to_display(),
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

    fn as_uint_range(&self) -> Option<DiscreteParamRange> {
        if let Self::UIntRange(param) = self {
            Some(DiscreteParamRange::new(param.low_value, param.high_value))
        } else {
            None
        }
    }

    fn uint_range(&self) -> DiscreteParamRange {
        self.as_uint_range().unwrap()
    }

    fn as_float_range(&self) -> Option<ContParamRange> {
        if let Self::FloatRange(param) = self {
            Some(ContParamRange::new(param.low_value, param.high_value))
        } else {
            None
        }
    }

    fn float_range(&self) -> ContParamRange {
        self.as_float_range().unwrap()
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

impl From<MetaUIntRangeParam> for MetaNodeParam {
    fn from(param: MetaUIntRangeParam) -> Self {
        Self::UIntRange(param)
    }
}

impl From<MetaFloatRangeParam> for MetaNodeParam {
    fn from(param: MetaFloatRangeParam) -> Self {
        Self::FloatRange(param)
    }
}

impl Default for MetaNodeParam {
    fn default() -> Self {
        Self::UInt(MetaUIntParam {
            text: LabelAndHoverText::label_only(""),
            value: 0,
            speed: 0.0,
        })
    }
}

impl MetaUIntParam {
    const fn new(text: LabelAndHoverText, value: u32) -> Self {
        Self {
            text,
            value,
            speed: 0.05,
        }
    }

    fn show_controls(&mut self, ui: &mut Ui) -> Response {
        option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value)
                .fixed_decimals(0)
                .speed(self.speed),
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

impl MetaUIntRangeParam {
    const fn new(text: LabelAndHoverText, low_value: u32, high_value: u32) -> Self {
        Self {
            text,
            low_value,
            high_value,
            speed: 0.05,
        }
    }

    const fn new_single_value(text: LabelAndHoverText, value: u32) -> Self {
        Self::new(text, value, value)
    }

    fn show_controls(&mut self, ui: &mut Ui) -> Response {
        labeled_option(ui, self.text.clone(), |ui| {
            run_drag_values_for_range(
                ui,
                &mut self.low_value,
                &mut self.high_value,
                0,
                u32::MAX,
                self.speed,
            )
        })
    }

    fn text_to_display(&self) -> String {
        if self.low_value == self.high_value {
            format!("{} = {}", self.text.label, self.low_value)
        } else {
            format!(
                "{} from {} to {}",
                self.text.label, self.low_value, self.high_value
            )
        }
    }
}

impl MetaFloatRangeParam {
    const fn new(text: LabelAndHoverText, low_value: f32, high_value: f32) -> Self {
        Self {
            text,
            low_value,
            high_value,
            min_value: f32::NEG_INFINITY,
            max_value: f32::INFINITY,
            speed: 0.05,
        }
    }

    const fn new_single_value(text: LabelAndHoverText, value: f32) -> Self {
        Self::new(text, value, value)
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
        labeled_option(ui, self.text.clone(), |ui| {
            run_drag_values_for_range(
                ui,
                &mut self.low_value,
                &mut self.high_value,
                self.min_value,
                self.max_value,
                self.speed,
            )
        })
    }

    fn text_to_display(&self) -> String {
        if self.low_value == self.high_value {
            format!("{} = {}", self.text.label, self.low_value)
        } else {
            format!(
                "{} from {} to {}",
                self.text.label, self.low_value, self.high_value
            )
        }
    }
}

impl CollapsedMetaSubtree {
    fn clear(&mut self) {
        self.size = Vec2::ZERO;
        self.exposed_parent_ports.clear();
        self.exposed_child_ports.clear();
    }

    fn port_position(&self, node_rect: &Rect, port: MetaPort) -> Pos2 {
        match port {
            MetaPort::Parent { slot, .. } => {
                MetaPort::parent_center(node_rect, slot, self.exposed_parent_ports.len())
            }
            MetaPort::Child { slot, .. } => {
                MetaPort::child_center(node_rect, slot, self.exposed_child_ports.len())
            }
        }
    }
}

fn run_drag_values_for_range<Num: Numeric>(
    ui: &mut Ui,
    low_value: &mut Num,
    high_value: &mut Num,
    min_value: Num,
    max_value: Num,
    speed: impl Into<f64> + Copy,
) -> Response {
    let mut low_drag_value = DragValue::new(low_value)
        .range(min_value..=max_value)
        .speed(speed);
    if Num::INTEGRAL {
        low_drag_value = low_drag_value.fixed_decimals(0);
    }
    let low_response = ui.add(low_drag_value);

    let to_label_response = ui.label("to");

    let mut high_drag_value = DragValue::new(high_value)
        .range(min_value..=max_value)
        .speed(speed);
    if Num::INTEGRAL {
        high_drag_value = high_drag_value.fixed_decimals(0);
    }
    let high_response = ui.add(high_drag_value);

    // If user moved low past high, push high up
    if low_response.changed() && *low_value > *high_value {
        *high_value = *low_value;
    }
    // If user moved high below low, push low down
    if high_response.changed() && *high_value < *low_value {
        *low_value = *high_value;
    }

    low_response.union(to_label_response).union(high_response)
}

fn prepare_text(ui: &Ui, text: String, font_id: FontId, color: Color32) -> Arc<Galley> {
    ui.fonts(|f| f.layout_no_wrap(text, font_id, color))
}

fn show_port(
    ui: &mut Ui,
    painter: &Painter,
    unique_port_id: Id,
    position: Pos2,
    enabled: bool,
    zoom: f32,
    cursor_hidden: bool,
    shape: MetaPortShape,
    color: Color32,
    label: &str,
) -> Response {
    let port_hit_diameter = 2.0 * PORT_HIT_RADIUS * zoom;
    let hit_rect = Rect::from_center_size(position, vec2(port_hit_diameter, port_hit_diameter));

    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let mut response = ui.interact(hit_rect, unique_port_id, sense);

    response = response.on_hover_text(label);

    if enabled && !cursor_hidden {
        response = response.on_hover_cursor(CursorIcon::PointingHand);
    }

    let port_radius = if enabled && response.hovered() {
        PORT_HOVER_RADIUS * zoom
    } else {
        PORT_RADIUS * zoom
    };

    match shape {
        MetaPortShape::Circle => {
            painter.circle_filled(position, port_radius, color);
        }
        MetaPortShape::Square => {
            let rect_size = 2.0 * port_radius;
            let rect = Rect::from_center_size(position, vec2(rect_size, rect_size));
            painter.rect_filled(rect, 2.0, color);
        }
    }

    response
}
