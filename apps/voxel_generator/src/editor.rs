use impact::{
    egui::{
        Align2, Button, Color32, ComboBox, Context, CursorIcon, DragValue, FontId, Id, Key,
        Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Window, vec2,
    },
    engine::Engine,
};
use impact_dev_ui::{
    CustomPanels, UserInterfaceConfig as DevUserInterfaceConfig,
    option_panels::{option_group, option_panel},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// TODO:
// Remap attached child links when changing to node kind with fewer child slots.
// Zoom and pan.
// Content-aware node size.

const CANVAS_DEFAULT_SIZE: Vec2 = vec2(800.0, 600.0);

const NODE_SIZE: Vec2 = vec2(160.0, 90.0);

const NODE_CORNER_RADIUS: f32 = 8.0;
const NODE_FILL_COLOR: Color32 = Color32::from_gray(42);
const NODE_STROKE: Stroke = Stroke {
    width: 1.0,
    color: Color32::WHITE,
};
const SELECTED_NODE_STROKE: Stroke = Stroke {
    width: 2.0,
    color: Color32::YELLOW,
};

const NODE_TEXT_COLOR: Color32 = Color32::WHITE;
const NODE_HEADER_FONT: FontId = FontId::proportional(14.0);
const NODE_PARAMS_FONT: FontId = FontId::proportional(12.0);

const PORT_RADIUS: f32 = 6.0;
const PORT_FILL_COLOR: Color32 = Color32::LIGHT_GRAY;
const HOVERED_PORT_FILL_COLOR: Color32 = Color32::WHITE;
const DISABLED_PORT_FILL_COLOR: Color32 = Color32::from_gray(80);

const LINK_STROKE: Stroke = Stroke {
    width: 2.0,
    color: Color32::WHITE,
};
const PENDING_LINK_STROKE: Stroke = Stroke {
    width: 2.0,
    color: Color32::LIGHT_GRAY,
};

const ROOT_INITIAL_POSITION: Pos2 = Pos2 {
    x: 0.5 * CANVAS_DEFAULT_SIZE.x - 0.5 * NODE_SIZE.x,
    y: 0.5 * NODE_SIZE.y,
};

#[derive(Clone, Debug)]
pub struct Editor {
    nodes: BTreeMap<NodeID, Node>,
    selected_node_id: Option<NodeID>,
    kind_to_add: NodeKind,
    pending_link: Option<PendingLink>,
    node_id_counter: NodeID,
    config: EditorConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub show_editor: bool,
}

type NodeID = u64;

#[derive(Clone, Debug)]
struct Node {
    position: Pos2,
    kind: NodeKind,
    params: Vec<NodeParam>,
    parent: Option<NodeID>,
    children: Vec<Option<NodeID>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum NodeKind {
    Output,
    #[default]
    Box,
    Sphere,
    GradientNoise,
    Translation,
    Rotation,
    Scaling,
    MultifractalNoise,
    MultiscaleSphere,
    Union,
    Subtraction,
    Intersection,
}

#[derive(Clone, Copy, Debug)]
struct PortConfig {
    has_parent: bool,
    children: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Port {
    Parent,
    Child { slot: usize, of: usize },
}

#[derive(Clone, Debug)]
struct PendingLink {
    from_node: NodeID,
    from_port: Port,
}

#[derive(Clone, Debug)]
enum NodeParam {
    Int(IntParam),
    Float(FloatParam),
}

#[derive(Clone, Debug)]
struct IntParam {
    label: &'static str,
    value: u32,
}

#[derive(Clone, Debug)]
struct FloatParam {
    label: &'static str,
    value: f32,
}

impl Editor {
    pub fn new(config: EditorConfig) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(0, Node::new(ROOT_INITIAL_POSITION, NodeKind::Output));
        Self {
            nodes,
            selected_node_id: None,
            kind_to_add: NodeKind::default(),
            pending_link: None,
            node_id_counter: 1,
            config,
        }
    }

    fn node(&self, node_id: NodeID) -> &Node {
        self.nodes.get(&node_id).unwrap()
    }

    fn node_mut(&mut self, node_id: NodeID) -> &mut Node {
        self.nodes.get_mut(&node_id).unwrap()
    }

    fn get_attached_node_and_port(&self, node_id: NodeID, port: Port) -> Option<(NodeID, Port)> {
        let node = self.nodes.get(&node_id)?;
        let attached_node_id = node.get_node_attached_to_port(port)?;
        let attached_node = self.nodes.get(&attached_node_id)?;
        let attached_port = attached_node.get_port_node_is_attached_to(node_id, port)?;
        Some((attached_node_id, attached_port))
    }

    fn node_can_reach_other(&self, node_id: NodeID, other_node_id: NodeID) -> bool {
        let mut stack = vec![node_id];
        let mut seen = BTreeSet::new();

        while let Some(node_id) = stack.pop() {
            if !seen.insert(node_id) {
                continue;
            }
            if node_id == other_node_id {
                return true;
            }
            if let Some(node) = self.nodes.get(&node_id) {
                for child_node_id in node.children.iter().filter_map(|id| *id) {
                    stack.push(child_node_id);
                }
            }
        }
        false
    }

    fn remove_node(&mut self, node_id: NodeID) {
        self.detach_parent_of(node_id);

        let n_child_slots = self
            .nodes
            .get(&node_id)
            .map_or(0, |node| node.kind.port_config().children);

        for slot in 0..n_child_slots {
            self.detach_child_slot(node_id, slot);
        }

        self.nodes.remove(&node_id);

        self.clear_pending_link_if_from(node_id);
    }

    fn clear_pending_link_if_from(&mut self, node_id: NodeID) {
        if self
            .pending_link
            .as_ref()
            .is_some_and(|pending_link| pending_link.from_node == node_id)
        {
            self.pending_link = None;
        }
    }

    fn can_attach(&self, parent_node_id: NodeID, child_node_id: NodeID, child_slot: usize) -> bool {
        if parent_node_id == child_node_id {
            return false;
        }

        // If they are already connected, attaching would create a cycle
        if self.node_can_reach_other(child_node_id, parent_node_id) {
            return false;
        }

        // Child must have no parent
        if self
            .nodes
            .get(&child_node_id)
            .and_then(|child_node| child_node.parent)
            .is_some()
        {
            return false;
        }

        // Slot must exist and be empty
        if !self
            .nodes
            .get(&parent_node_id)
            .and_then(|p| p.children.get(child_slot))
            .is_some_and(|slot| slot.is_none())
        {
            return false;
        }

        true
    }

    fn try_attach(
        &mut self,
        parent_node_id: NodeID,
        child_node_id: NodeID,
        child_slot: usize,
    ) -> bool {
        if !self.can_attach(parent_node_id, child_node_id, child_slot) {
            return false;
        }
        if let Some(child_node) = self.nodes.get_mut(&child_node_id) {
            child_node.parent = Some(parent_node_id);
        }
        if let Some(parent_node) = self.nodes.get_mut(&parent_node_id) {
            parent_node.children[child_slot] = Some(child_node_id);
        }
        true
    }

    fn detach_parent_of(&mut self, node_id: NodeID) {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return;
        };
        let Some(parent_node_id) = node.parent.take() else {
            return;
        };
        let Some(parent_node) = self.nodes.get_mut(&parent_node_id) else {
            return;
        };
        if let Some(slot) = parent_node
            .children
            .iter_mut()
            .find(|child| **child == Some(node_id))
        {
            *slot = None;
        }
    }

    fn detach_child_slot(&mut self, node_id: NodeID, slot: usize) {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return;
        };
        let Some(child_node_id) = node.children.get_mut(slot).and_then(|child| child.take()) else {
            return;
        };
        let Some(child_node) = self.nodes.get_mut(&child_node_id) else {
            return;
        };
        if child_node.parent == Some(node_id) {
            child_node.parent = None;
        }
    }
}

impl CustomPanels for Editor {
    fn run_toolbar_buttons(&mut self, ui: &mut Ui) {
        ui.toggle_value(&mut self.config.show_editor, "Voxel editor");
    }

    fn run_panels(&mut self, ctx: &Context, config: &DevUserInterfaceConfig, _engine: &Engine) {
        if !self.config.show_editor {
            return;
        }

        option_panel(ctx, config, "Editor panel", |ui| {
            option_group(ui, "creation", |ui| {
                ComboBox::from_id_salt("kind_to_add")
                    .selected_text(self.kind_to_add.label())
                    .show_ui(ui, |ui| {
                        for kind_option in NodeKind::all_non_root() {
                            ui.selectable_value(
                                &mut self.kind_to_add,
                                kind_option,
                                kind_option.label(),
                            );
                        }
                    });

                if ui.button("Add node").clicked() {
                    let position =
                        self.nodes.values().last().unwrap().position + vec2(0.0, 1.5 * NODE_SIZE.y);

                    self.nodes
                        .insert(self.node_id_counter, Node::new(position, self.kind_to_add));

                    self.node_id_counter += 1;
                }
            });

            option_group(ui, "modification", |ui| {
                let mut id_to_delete = None;

                if let Some(selected_id) = self.selected_node_id {
                    let mut node = self.nodes.get_mut(&selected_id).unwrap();
                    let mut kind = node.kind;

                    ComboBox::from_id_salt("selected_kind")
                        .selected_text(node.kind.label())
                        .show_ui(ui, |ui| {
                            for kind_option in NodeKind::all_non_root() {
                                ui.selectable_value(&mut kind, kind_option, kind_option.label());
                            }
                        });

                    if kind != node.kind {
                        // Detach dropped children cleanly
                        let old_child_slots = node.children.len();
                        let new_child_slots = kind.port_config().children;
                        if new_child_slots < old_child_slots {
                            for slot in new_child_slots..old_child_slots {
                                self.detach_child_slot(selected_id, slot);
                            }
                        }
                        node = self.node_mut(selected_id);
                        node.change_kind(kind);
                    }

                    ui.end_row();

                    for param in &mut node.params {
                        param.show_controls(ui);
                    }

                    if ui.button("Delete selected").clicked() {
                        id_to_delete = Some(selected_id);
                    }
                    ui.end_row();
                } else {
                    ui.add_enabled(false, Button::new("Delete selected"));
                    ui.end_row();
                }

                if let Some(id) = id_to_delete {
                    if self.selected_node_id == Some(id) {
                        self.selected_node_id = None;
                    }
                    self.remove_node(id);
                }
            });
        });

        Window::new("Generator graph")
            .default_size(CANVAS_DEFAULT_SIZE)
            .vscroll(false)
            .hscroll(false)
            .show(ctx, |ui| {
                let (canvas_rect, canvas_resp) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click());

                let origin = canvas_rect.min;

                let painter = ui.painter_at(canvas_rect);

                if canvas_resp.clicked() {
                    if self.pending_link.is_some() {
                        self.pending_link = None;
                    } else if self.selected_node_id.is_some() {
                        self.selected_node_id = None;
                    }
                }

                let mut node_rects = BTreeMap::<NodeID, Rect>::new();

                for (&node_id, node) in &mut self.nodes {
                    let mut pos = node.position;

                    let node_rect = Rect::from_min_size(origin + pos.to_vec2(), NODE_SIZE);
                    node_rects.insert(node_id, node_rect);

                    let node_response = ui.interact(
                        node_rect,
                        Id::new(("node", node_id)),
                        Sense::click_and_drag(),
                    );

                    // Handle node selection

                    if node_response.clicked()
                        && node.kind.is_selectable()
                        && self.pending_link.is_none()
                    {
                        self.selected_node_id = Some(node_id);
                    }

                    let is_selected = self.selected_node_id == Some(node_id);

                    // Handle node dragging

                    if node_response.dragged() {
                        pos += node_response.drag_delta();

                        // Clamp inside canvas
                        pos.x = pos
                            .x
                            .clamp(0.0, (canvas_rect.width() - NODE_SIZE.x).max(0.0));
                        pos.y = pos
                            .y
                            .clamp(0.0, (canvas_rect.height() - NODE_SIZE.y).max(0.0));

                        node.position = pos;
                    }

                    // Draw node background and outline

                    let stroke = if is_selected {
                        SELECTED_NODE_STROKE
                    } else {
                        NODE_STROKE
                    };
                    painter.rect_filled(node_rect, NODE_CORNER_RADIUS, NODE_FILL_COLOR);
                    painter.rect_stroke(node_rect, NODE_CORNER_RADIUS, stroke, StrokeKind::Inside);

                    // Draw node text

                    let anchor = Align2::CENTER_TOP;
                    let mut offset_y = 16.0;

                    painter.text(
                        node_rect.center_top() + vec2(0.0, offset_y),
                        anchor,
                        node.kind.label(),
                        NODE_HEADER_FONT,
                        NODE_TEXT_COLOR,
                    );
                    offset_y += 26.0;

                    for param in &node.params {
                        painter.text(
                            node_rect.center_top() + vec2(0.0, offset_y),
                            anchor,
                            param.text_to_display(),
                            NODE_PARAMS_FONT.clone(),
                            NODE_TEXT_COLOR,
                        );
                        offset_y += 16.0;
                    }
                }

                // Draw ports

                for (&node_id, node_rect) in &node_rects {
                    for port in self.node(node_id).kind.port_config().ports() {
                        let mut enabled = true;
                        let mut highlighted = false;

                        if let Some(pending_link) = &self.pending_link {
                            // Ports we can attach the pending link to are
                            // enabled and highlighted
                            match (pending_link.from_port, port) {
                                (Port::Parent, Port::Child { slot, .. }) => {
                                    let child_node_id = pending_link.from_node;
                                    let parent_node_id = node_id;
                                    enabled = self.can_attach(parent_node_id, child_node_id, slot);
                                    highlighted = enabled;
                                }
                                (Port::Child { slot, .. }, Port::Parent) => {
                                    let parent_node_id = pending_link.from_node;
                                    let child_node_id = node_id;
                                    enabled = self.can_attach(parent_node_id, child_node_id, slot);
                                    highlighted = enabled;
                                }
                                _ => {
                                    // Mismatched ports are disabled
                                    enabled = false;
                                    highlighted = false;
                                }
                            }
                        }

                        let response =
                            port.show(ui, &painter, node_id, node_rect, enabled, highlighted);

                        if response.clicked() {
                            // Detach if there is a node attached to the port
                            if self.pending_link.is_none()
                                && let Some((attached_node_id, attached_port)) =
                                    self.get_attached_node_and_port(node_id, port)
                            {
                                match port {
                                    Port::Parent => {
                                        self.detach_parent_of(node_id);
                                    }
                                    Port::Child { slot, .. } => {
                                        self.detach_child_slot(node_id, slot);
                                    }
                                }

                                // Create a pending link from the remaining attached port
                                self.pending_link = Some(PendingLink {
                                    from_node: attached_node_id,
                                    from_port: attached_port,
                                });
                                continue;
                            }

                            if let Some(pending_link) = &self.pending_link {
                                match (pending_link.from_port, port) {
                                    (Port::Parent, Port::Child { slot, .. }) => {
                                        let child_node_id = pending_link.from_node;
                                        let parent_node_id = node_id;
                                        if self.try_attach(parent_node_id, child_node_id, slot) {
                                            self.pending_link = None;
                                        }
                                    }
                                    (Port::Child { slot, .. }, Port::Parent) => {
                                        let parent_node_id = pending_link.from_node;
                                        let child_node_id = node_id;
                                        if self.try_attach(parent_node_id, child_node_id, slot) {
                                            self.pending_link = None;
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                self.pending_link = Some(PendingLink {
                                    from_node: node_id,
                                    from_port: port,
                                });
                            }
                        }
                    }
                }

                // Handle cancellation of pending link or node deletion with keyboard

                if ui.input(|i| i.key_pressed(Key::Delete)) {
                    if self.pending_link.is_some() {
                        self.pending_link = None;
                    } else if let Some(selected_id) = self.selected_node_id {
                        self.selected_node_id = None;
                        self.remove_node(selected_id);
                    }
                }

                // Draw edges

                for (&node_id, node) in &self.nodes {
                    if let Some(parent_node_id) = node.parent
                        && let (Some(parent_rect), Some(node_rect)) =
                            (node_rects.get(&parent_node_id), node_rects.get(&node_id))
                    {
                        let Some(parent_node) = self.nodes.get(&parent_node_id) else {
                            continue;
                        };

                        let Some(slot) = parent_node
                            .children
                            .iter()
                            .position(|child| *child == Some(node_id))
                        else {
                            continue;
                        };

                        let from = Port::Child {
                            slot,
                            of: parent_node.children.len(),
                        }
                        .center(parent_rect);

                        let to = Port::Parent.center(node_rect);

                        painter.line_segment([from, to], LINK_STROKE);
                    }
                }

                // Draw pending link

                if let Some(pending_link) = &self.pending_link
                    && let (Some(node_rect), Some(mouse_pos)) = (
                        node_rects.get(&pending_link.from_node),
                        ui.input(|i| i.pointer.hover_pos()),
                    )
                {
                    painter.line_segment(
                        [pending_link.from_port.center(node_rect), mouse_pos],
                        PENDING_LINK_STROKE,
                    );
                }
            });
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { show_editor: true }
    }
}

impl Node {
    fn new(position: Pos2, kind: NodeKind) -> Self {
        Self {
            position,
            kind,
            params: kind.default_params(),
            parent: None,
            children: vec![None; kind.port_config().children],
        }
    }

    fn change_kind(&mut self, new_kind: NodeKind) {
        self.kind = new_kind;
        self.params = new_kind.default_params();
        self.children.resize(new_kind.port_config().children, None);
    }

    fn get_node_attached_to_port(&self, port: Port) -> Option<NodeID> {
        match port {
            Port::Parent => self.parent,
            Port::Child { slot, .. } => self.children.get(slot).copied().flatten(),
        }
    }

    fn get_port_node_is_attached_to(
        &self,
        other_node_id: NodeID,
        other_port: Port,
    ) -> Option<Port> {
        match other_port {
            Port::Parent => self
                .children
                .iter()
                .position(|child| *child == Some(other_node_id))
                .map(|slot| Port::Child {
                    slot,
                    of: self.children.len(),
                }),
            Port::Child { .. } => (self.parent == Some(other_node_id)).then_some(Port::Parent),
        }
    }
}

impl NodeKind {
    const fn all_non_root() -> [Self; 11] {
        [
            Self::Box,
            Self::Sphere,
            Self::GradientNoise,
            Self::Translation,
            Self::Rotation,
            Self::Scaling,
            Self::MultifractalNoise,
            Self::MultiscaleSphere,
            Self::Union,
            Self::Subtraction,
            Self::Intersection,
        ]
    }

    const fn label(&self) -> &'static str {
        match self {
            Self::Output => "Output",
            Self::Box => "Box",
            Self::Sphere => "Sphere",
            Self::GradientNoise => "Gradient noise",
            Self::Translation => "Translation",
            Self::Rotation => "Rotation",
            Self::Scaling => "Scaling",
            Self::MultifractalNoise => "Multifractal noise",
            Self::MultiscaleSphere => "Multiscale sphere",
            Self::Union => "Union",
            Self::Subtraction => "Subtraction",
            Self::Intersection => "Intersection",
        }
    }

    const fn port_config(&self) -> PortConfig {
        match self {
            Self::Output => PortConfig::root(),
            Self::Box | Self::Sphere | Self::GradientNoise => PortConfig::leaf(),
            Self::Translation
            | Self::Rotation
            | Self::Scaling
            | Self::MultifractalNoise
            | Self::MultiscaleSphere => PortConfig::unary(),
            Self::Union | Self::Subtraction | Self::Intersection => PortConfig::binary(),
        }
    }

    fn is_selectable(&self) -> bool {
        *self != Self::Output
    }

    fn default_params(&self) -> Vec<NodeParam> {
        match self {
            Self::Output => vec![],
            Self::Box => vec![
                FloatParam::new("extent_x", 1.0).into(),
                FloatParam::new("extent_y", 1.0).into(),
                FloatParam::new("extent_z", 1.0).into(),
            ],
            Self::Sphere => vec![FloatParam::new("radius", 1.0).into()],
            Self::GradientNoise => vec![
                FloatParam::new("extent_x", 1.0).into(),
                FloatParam::new("extent_y", 1.0).into(),
                FloatParam::new("extent_z", 1.0).into(),
                FloatParam::new("noise_frequency", 1.0).into(),
                FloatParam::new("noise_threshold", 1.0).into(),
                IntParam::new("seed", 0).into(),
            ],
            Self::Translation => vec![
                FloatParam::new("x", 0.0).into(),
                FloatParam::new("y", 0.0).into(),
                FloatParam::new("z", 0.0).into(),
            ],
            Self::Rotation => vec![
                FloatParam::new("angle", 0.0).into(),
                FloatParam::new("axis_x", 0.0).into(),
                FloatParam::new("axis_y", 0.0).into(),
                FloatParam::new("axis_z", 1.0).into(),
            ],
            Self::Scaling => vec![FloatParam::new("factor", 1.0).into()],
            Self::MultifractalNoise => vec![
                IntParam::new("octaves", 0).into(),
                FloatParam::new("frequency", 1.0).into(),
                FloatParam::new("lacunarity", 1.0).into(),
                FloatParam::new("persistence", 1.0).into(),
                FloatParam::new("amplitude", 1.0).into(),
                IntParam::new("seed", 0).into(),
            ],
            Self::MultiscaleSphere => vec![
                IntParam::new("octaves", 0).into(),
                FloatParam::new("max_scale", 1.0).into(),
                FloatParam::new("persistence", 1.0).into(),
                FloatParam::new("inflation", 1.0).into(),
                FloatParam::new("smoothness", 1.0).into(),
                IntParam::new("seed", 0).into(),
            ],
            Self::Union | Self::Subtraction | Self::Intersection => {
                vec![FloatParam::new("smoothness", 1.0).into()]
            }
        }
    }
}

impl PortConfig {
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

    fn ports(&self) -> impl Iterator<Item = Port> {
        self.has_parent
            .then_some(Port::Parent)
            .into_iter()
            .chain((0..self.children).map(|slot| Port::Child {
                slot,
                of: self.children,
            }))
    }
}

impl Port {
    fn center(&self, node_rect: &Rect) -> Pos2 {
        match self {
            Self::Parent => node_rect.center_top(),
            &Self::Child { slot, of } => {
                node_rect.left_bottom()
                    + vec2((1.0 + slot as f32) * NODE_SIZE.x / (of as f32 + 1.0), 0.0)
            }
        }
    }

    fn id(&self, node_id: NodeID) -> Id {
        match self {
            Self::Parent => Id::new(("parent_port", node_id)),
            &Self::Child { slot, .. } => Id::new(("child_port", slot, node_id)),
        }
    }

    fn show(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        node_id: NodeID,
        node_rect: &Rect,
        enabled: bool,
        highlighted: bool,
    ) -> Response {
        let center = self.center(node_rect);

        let hit_rect = Rect::from_center_size(center, vec2(2.0 * PORT_RADIUS, 2.0 * PORT_RADIUS));

        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(hit_rect, self.id(node_id), sense);

        if enabled {
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

        painter.circle_filled(center, PORT_RADIUS, color);

        response
    }
}

impl NodeParam {
    fn show_controls(&mut self, ui: &mut Ui) {
        match self {
            NodeParam::Int(param) => param.show_controls(ui),
            NodeParam::Float(param) => param.show_controls(ui),
        }
        ui.end_row();
    }

    fn text_to_display(&self) -> String {
        match self {
            NodeParam::Int(param) => param.text_to_display(),
            NodeParam::Float(param) => param.text_to_display(),
        }
    }
}

impl From<IntParam> for NodeParam {
    fn from(param: IntParam) -> Self {
        Self::Int(param)
    }
}

impl From<FloatParam> for NodeParam {
    fn from(param: FloatParam) -> Self {
        Self::Float(param)
    }
}

impl IntParam {
    fn new(label: &'static str, value: u32) -> Self {
        Self { label, value }
    }

    fn show_controls(&mut self, ui: &mut Ui) {
        ui.label(self.label);
        ui.add(DragValue::new(&mut self.value).speed(1));
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.label, self.value)
    }
}

impl FloatParam {
    fn new(label: &'static str, value: f32) -> Self {
        Self { label, value }
    }

    fn show_controls(&mut self, ui: &mut Ui) {
        ui.label(self.label);
        ui.add(DragValue::new(&mut self.value).speed(0.1));
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.label, self.value)
    }
}
