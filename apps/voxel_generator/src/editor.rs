use impact::{
    egui::{
        Button, Color32, ComboBox, Context, CursorIcon, DragValue, FontId, Galley, Id, Key,
        Painter, PointerButton, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Window,
        pos2, vec2,
    },
    engine::Engine,
};
use impact_dev_ui::{
    CustomPanels, UserInterfaceConfig as DevUserInterfaceConfig,
    option_panels::{
        LabelAndHoverText, labeled_option, option_drag_value, option_group, option_panel,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const CANVAS_DEFAULT_SIZE: Vec2 = vec2(800.0, 600.0);

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

const MIN_NODE_SEPARATION: f32 = 8.0;
const NEW_NODE_GAP: f32 = 32.0;

const PORT_RADIUS: f32 = 6.0;
const PORT_FILL_COLOR: Color32 = Color32::LIGHT_GRAY;
const HOVERED_PORT_FILL_COLOR: Color32 = Color32::WHITE;
const DISABLED_PORT_FILL_COLOR: Color32 = Color32::from_gray(80);

const EDGE_WIDTH: f32 = 2.0;
const PENDING_EDGE_WIDTH: f32 = 2.0;
const EDGE_COLOR: Color32 = Color32::WHITE;
const PENDING_EDGE_COLOR: Color32 = Color32::LIGHT_GRAY;

const SCROLL_SENSITIVITY: f32 = 4e-3;
const MIN_ZOOM: f32 = 0.3;
const MAX_ZOOM: f32 = 3.0;

#[derive(Clone, Debug)]
pub struct Editor {
    canvas: Canvas,
    kind_to_add: NodeKind,
    node_id_counter: NodeID,
    config: EditorConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub show_editor: bool,
}

#[derive(Clone, Debug)]
struct Canvas {
    state: CanvasState,
    nodes: BTreeMap<NodeID, Node>,
    selected_node_id: Option<NodeID>,
    pending_edge: Option<PendingEdge>,
    is_panning: bool,
    dragging_node_id: Option<NodeID>,
}

#[derive(Clone, Debug)]
struct CanvasState {
    pan: Vec2,
    zoom: f32,
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
struct PendingEdge {
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
    text: LabelAndHoverText,
    value: u32,
}

#[derive(Clone, Debug)]
struct FloatParam {
    text: LabelAndHoverText,
    value: f32,
}

impl Editor {
    pub fn new(config: EditorConfig) -> Self {
        Self {
            canvas: Canvas::new(),
            kind_to_add: NodeKind::default(),
            node_id_counter: 0,
            config,
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

        let mut pending_new_node = if self.canvas.nodes.is_empty() {
            let output_node_id = self.node_id_counter;
            self.node_id_counter += 1;
            Some((output_node_id, NodeKind::Output))
        } else {
            None
        };

        option_panel(ctx, config, "Editor panel", |ui| {
            option_group(ui, "creation", |ui| {
                if ui
                    .add_enabled(pending_new_node.is_none(), Button::new("Add node"))
                    .clicked()
                {
                    pending_new_node = Some((self.node_id_counter, self.kind_to_add));
                    self.node_id_counter += 1;
                }

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
            });

            option_group(ui, "modification", |ui| {
                let mut id_of_node_to_delete = None;

                if let Some(selected_node_id) = self.canvas.selected_node_id {
                    let mut selected_node = self.canvas.nodes.get_mut(&selected_node_id).unwrap();
                    let mut kind = selected_node.kind;

                    labeled_option(
                        ui,
                        LabelAndHoverText {
                            label: "Kind",
                            hover_text: "",
                        },
                        |ui| {
                            ComboBox::from_id_salt("selected_kind")
                                .selected_text(selected_node.kind.label())
                                .show_ui(ui, |ui| {
                                    for kind_option in NodeKind::all_non_root() {
                                        ui.selectable_value(
                                            &mut kind,
                                            kind_option,
                                            kind_option.label(),
                                        );
                                    }
                                })
                        },
                    );

                    if kind != selected_node.kind {
                        self.canvas.change_node_kind(selected_node_id, kind);
                        selected_node = self.canvas.node_mut(selected_node_id);
                    }

                    for param in &mut selected_node.params {
                        param.show_controls(ui);
                    }

                    if ui.button("Delete node").clicked() {
                        id_of_node_to_delete = Some(selected_node_id);
                    }
                    ui.end_row();
                } else {
                    ui.add_enabled(false, Button::new("Delete node"));
                    ui.end_row();
                }

                if let Some(id) = id_of_node_to_delete {
                    self.canvas.remove_node(id);
                }
            });

            self.canvas.show(ctx, pending_new_node);
        });
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { show_editor: true }
    }
}

impl Canvas {
    fn new() -> Self {
        Self {
            state: CanvasState::new(),
            nodes: BTreeMap::new(),
            selected_node_id: None,
            pending_edge: None,
            is_panning: false,
            dragging_node_id: None,
        }
    }

    fn cursor_should_be_hidden(&self) -> bool {
        self.is_panning || self.dragging_node_id.is_some()
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
        if self.selected_node_id == Some(node_id) {
            self.selected_node_id = None;
        }

        self.detach_parent_of(node_id);

        let n_child_slots = self
            .nodes
            .get(&node_id)
            .map_or(0, |node| node.kind.port_config().children);

        for slot in 0..n_child_slots {
            self.detach_child_slot(node_id, slot);
        }

        self.nodes.remove(&node_id);

        self.clear_pending_edge_if_from(node_id);
    }

    fn change_node_kind(&mut self, node_id: NodeID, new_kind: NodeKind) {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return;
        };
        let n_old_child_slots = node.children.len();
        let n_new_child_slots = new_kind.port_config().children;

        // Detach dropped children and re-attach if there are available slots
        if n_new_child_slots < n_old_child_slots {
            for slot in n_new_child_slots..n_old_child_slots {
                if let Some(child_node_id) = self.detach_child_slot(node_id, slot) {
                    for slot in 0..n_new_child_slots {
                        if self.node(node_id).children[slot].is_none() {
                            self.try_attach(node_id, child_node_id, slot);
                        }
                    }
                };
            }
        }

        self.node_mut(node_id).change_kind(new_kind);
    }

    fn clear_pending_edge_if_from(&mut self, node_id: NodeID) {
        if self
            .pending_edge
            .as_ref()
            .is_some_and(|pending_edge| pending_edge.from_node == node_id)
        {
            self.pending_edge = None;
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

    /// Returns the ID of the detached parent node.
    fn detach_parent_of(&mut self, node_id: NodeID) -> Option<NodeID> {
        let node = self.nodes.get_mut(&node_id)?;

        let parent_node_id = node.parent.take()?;

        let parent_node = self.nodes.get_mut(&parent_node_id)?;

        if let Some(slot) = parent_node
            .children
            .iter_mut()
            .find(|child| **child == Some(node_id))
        {
            *slot = None;
            Some(parent_node_id)
        } else {
            None
        }
    }

    /// Returns the ID of the detached child node.
    fn detach_child_slot(&mut self, node_id: NodeID, slot: usize) -> Option<NodeID> {
        let node = self.nodes.get_mut(&node_id)?;

        let child_node_id = node.children.get_mut(slot).and_then(|child| child.take())?;

        let child_node = self.nodes.get_mut(&child_node_id)?;

        if child_node.parent == Some(node_id) {
            child_node.parent = None;
            Some(child_node_id)
        } else {
            None
        }
    }

    fn show(&mut self, ctx: &Context, pending_new_node: Option<(NodeID, NodeKind)>) {
        Window::new("Generator graph")
            .default_size(CANVAS_DEFAULT_SIZE)
            .vscroll(false)
            .hscroll(false)
            .show(ctx, |ui| {
                let (canvas_rect, canvas_response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

                let canvas_origin = canvas_rect.min;

                let painter = ui.painter_at(canvas_rect);

                if canvas_response.drag_started_by(PointerButton::Secondary) {
                    self.is_panning = true;
                }
                if canvas_response.drag_stopped_by(PointerButton::Secondary) {
                    self.is_panning = false;
                }
                self.state.handle_drag(&canvas_response);

                self.state.handle_scroll(ui, canvas_rect);

                if canvas_response.clicked() {
                    if self.pending_edge.is_some() {
                        self.pending_edge = None;
                    } else if self.selected_node_id.is_some() {
                        self.selected_node_id = None;
                    }
                }

                let mut world_node_rects = BTreeMap::<NodeID, Rect>::new();
                for (&node_id, node) in &self.nodes {
                    world_node_rects.insert(
                        node_id,
                        Rect::from_min_size(node.position, node.compute_size(ui, self.state.zoom)),
                    );
                }

                // Handle pending new node

                if let Some((node_id, kind)) = pending_new_node {
                    let (mut node, mut world_node_rect) =
                        if let Some(last_node) = self.nodes.values().last() {
                            let last_node_rect = world_node_rects.values().last().unwrap();
                            let position = last_node.position
                                + vec2(0.0, last_node_rect.height() + NEW_NODE_GAP);

                            let node = Node::new(position, kind);

                            let world_node_rect = Rect::from_min_size(
                                node.position,
                                node.compute_size(ui, self.state.zoom),
                            );

                            (node, world_node_rect)
                        } else {
                            let mut node = Node::new(Pos2::ZERO, kind);
                            let node_size = node.compute_size(ui, self.state.zoom);

                            node.position = self
                                .state
                                .screen_pos_to_world_space(canvas_origin, canvas_rect.center_top())
                                + vec2(-0.5 * node_size.x, 0.0);

                            let world_node_rect = Rect::from_min_size(node.position, node_size);

                            (node, world_node_rect)
                        };

                    let resolve_delta = compute_delta_to_resolve_overlaps(
                        &world_node_rects,
                        node_id,
                        world_node_rect,
                    );

                    world_node_rect = world_node_rect.translate(resolve_delta);
                    node.position += resolve_delta;

                    self.nodes.insert(node_id, node);
                    world_node_rects.insert(node_id, world_node_rect);
                }

                for ((&node_id, node), &world_node_rect) in
                    self.nodes.iter_mut().zip(world_node_rects.values())
                {
                    let node_rect = self
                        .state
                        .world_rect_to_screen_space(canvas_origin, world_node_rect);

                    let node_response = ui.interact(
                        node_rect,
                        Id::new(("node", node_id)),
                        Sense::click_and_drag(),
                    );

                    if node_response.drag_started() {
                        self.dragging_node_id = Some(node_id);
                    }
                    if node_response.drag_stopped() && self.dragging_node_id == Some(node_id) {
                        self.dragging_node_id = None;
                    }

                    // Handle node selection

                    if node_response.clicked()
                        && node.kind.is_selectable()
                        && self.pending_edge.is_none()
                    {
                        self.selected_node_id = Some(node_id);
                    }

                    let is_selected = self.selected_node_id == Some(node_id);

                    // Handle node dragging

                    if node_response.dragged() {
                        let delta = self
                            .state
                            .screen_vec_to_world_space(node_response.drag_delta());

                        let moved_node_rect = world_node_rect.translate(delta);
                        let resolve_delta = compute_delta_to_resolve_overlaps(
                            &world_node_rects,
                            node_id,
                            moved_node_rect,
                        );

                        node.position += delta + resolve_delta;
                    }

                    // Draw node background and outline

                    let (stroke_width, stroke_color) = if is_selected {
                        (SELECTED_NODE_BOUNDARY_WIDTH, SELECTED_NODE_BOUNDARY_COLOR)
                    } else {
                        (NODE_BOUNDARY_WIDTH, NODE_BOUNDARY_COLOR)
                    };
                    let stroke = Stroke {
                        width: stroke_width * self.state.zoom,
                        color: stroke_color,
                    };
                    let corner_radius = NODE_CORNER_RADIUS * self.state.zoom;
                    painter.rect_filled(node_rect, corner_radius, NODE_FILL_COLOR);
                    painter.rect_stroke(node_rect, corner_radius, stroke, StrokeKind::Inside);

                    // Draw node text
                    node.paint_text(ui, &painter, &node_rect, self.state.zoom);
                }

                // We will only need node rects in screen space from now
                for node_rect in world_node_rects.values_mut() {
                    *node_rect = self
                        .state
                        .world_rect_to_screen_space(canvas_origin, *node_rect);
                }
                let node_rects = world_node_rects;

                // Draw ports

                for (&node_id, node_rect) in &node_rects {
                    for port in self.node(node_id).kind.port_config().ports() {
                        let mut enabled = true;
                        let mut highlighted = false;

                        if let Some(pending_edge) = &self.pending_edge {
                            // Ports we can attach the pending edge to are
                            // enabled and highlighted
                            match (pending_edge.from_port, port) {
                                (Port::Parent, Port::Child { slot, .. }) => {
                                    let child_node_id = pending_edge.from_node;
                                    let parent_node_id = node_id;
                                    enabled = self.can_attach(parent_node_id, child_node_id, slot);
                                    highlighted = enabled;
                                }
                                (Port::Child { slot, .. }, Port::Parent) => {
                                    let parent_node_id = pending_edge.from_node;
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

                        let response = port.show(
                            ui,
                            &painter,
                            node_id,
                            node_rect,
                            enabled,
                            highlighted,
                            self.state.zoom,
                            self.cursor_should_be_hidden(),
                        );

                        if response.clicked() {
                            // Detach if there is a node attached to the port
                            if self.pending_edge.is_none()
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

                                // Create a pending edge from the remaining attached port
                                self.pending_edge = Some(PendingEdge {
                                    from_node: attached_node_id,
                                    from_port: attached_port,
                                });
                                continue;
                            }

                            if let Some(pending_edge) = &self.pending_edge {
                                match (pending_edge.from_port, port) {
                                    (Port::Parent, Port::Child { slot, .. }) => {
                                        let child_node_id = pending_edge.from_node;
                                        let parent_node_id = node_id;
                                        if self.try_attach(parent_node_id, child_node_id, slot) {
                                            self.pending_edge = None;
                                        }
                                    }
                                    (Port::Child { slot, .. }, Port::Parent) => {
                                        let parent_node_id = pending_edge.from_node;
                                        let child_node_id = node_id;
                                        if self.try_attach(parent_node_id, child_node_id, slot) {
                                            self.pending_edge = None;
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                self.pending_edge = Some(PendingEdge {
                                    from_node: node_id,
                                    from_port: port,
                                });
                            }
                        }
                    }
                }

                // Handle cancellation of pending edge or node deletion with keyboard

                if ui.input(|i| i.key_pressed(Key::Delete)) {
                    if self.pending_edge.is_some() {
                        self.pending_edge = None;
                    } else if let Some(selected_id) = self.selected_node_id {
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

                        painter.line_segment(
                            [from, to],
                            Stroke {
                                width: EDGE_WIDTH * self.state.zoom,
                                color: EDGE_COLOR,
                            },
                        );
                    }
                }

                // Draw pending edge

                if let Some(pending_edge) = &self.pending_edge
                    && let (Some(node_rect), Some(mouse_pos)) = (
                        node_rects.get(&pending_edge.from_node),
                        ui.input(|i| i.pointer.hover_pos()),
                    )
                {
                    painter.line_segment(
                        [pending_edge.from_port.center(node_rect), mouse_pos],
                        Stroke {
                            width: PENDING_EDGE_WIDTH * self.state.zoom,
                            color: PENDING_EDGE_COLOR,
                        },
                    );
                }

                if self.cursor_should_be_hidden() {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::None);
                }
            });
    }
}

impl CanvasState {
    fn new() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    fn world_pos_to_screen_space(&self, canvas_origin: Pos2, world_pos: Pos2) -> Pos2 {
        canvas_origin + self.pan + (world_pos.to_vec2() * self.zoom)
    }

    fn screen_pos_to_world_space(&self, canvas_origin: Pos2, screen_pos: Pos2) -> Pos2 {
        ((screen_pos - canvas_origin - self.pan) / self.zoom).to_pos2()
    }

    fn world_vec_to_screen_space(&self, world_vec: Vec2) -> Vec2 {
        world_vec * self.zoom
    }

    fn screen_vec_to_world_space(&self, screen_vec: Vec2) -> Vec2 {
        screen_vec / self.zoom
    }

    fn world_rect_to_screen_space(&self, canvas_origin: Pos2, world_rect: Rect) -> Rect {
        Rect::from_min_size(
            self.world_pos_to_screen_space(canvas_origin, world_rect.min),
            self.world_vec_to_screen_space(world_rect.size()),
        )
    }

    fn handle_drag(&mut self, canvas_response: &Response) {
        if canvas_response.dragged_by(PointerButton::Secondary) {
            self.pan += canvas_response.drag_delta();
        }
    }

    fn handle_scroll(&mut self, ui: &mut Ui, canvas_rect: Rect) {
        if ui.rect_contains_pointer(canvas_rect)
            && let Some(scroll) = ui.input(|i| {
                let delta = i.smooth_scroll_delta.y;
                (delta != 0.0).then_some(delta)
            })
            && let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos())
        {
            let mouse_world_pos_before = self.screen_pos_to_world_space(canvas_rect.min, mouse_pos);

            let factor = (1.0 + scroll * SCROLL_SENSITIVITY).max(0.5);
            let new_zoom = (factor * self.zoom).clamp(MIN_ZOOM, MAX_ZOOM);

            self.pan += mouse_world_pos_before.to_vec2() * (self.zoom - new_zoom);
            self.zoom = new_zoom;
        }
    }
}

impl Default for CanvasState {
    fn default() -> Self {
        Self::new()
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
                FloatParam::new(LabelAndHoverText::label_only("Extent in x"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Extent in y"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Extent in z"), 1.0).into(),
            ],
            Self::Sphere => {
                vec![FloatParam::new(LabelAndHoverText::label_only("Radius"), 1.0).into()]
            }
            Self::GradientNoise => vec![
                FloatParam::new(LabelAndHoverText::label_only("Extent in x"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Extent in y"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Extent in z"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Noise frequency"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Noise threshold"), 1.0).into(),
                IntParam::new(LabelAndHoverText::label_only("Seed"), 0).into(),
            ],
            Self::Translation => vec![
                FloatParam::new(LabelAndHoverText::label_only("In x"), 0.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("In y"), 0.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("In z"), 0.0).into(),
            ],
            Self::Rotation => vec![
                FloatParam::new(LabelAndHoverText::label_only("Angle"), 0.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Axis (x)"), 0.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Axis (y)"), 0.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Axis (z)"), 1.0).into(),
            ],
            Self::Scaling => {
                vec![FloatParam::new(LabelAndHoverText::label_only("Factor"), 1.0).into()]
            }
            Self::MultifractalNoise => vec![
                IntParam::new(LabelAndHoverText::label_only("Octaves"), 0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Frequency"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Lacunarity"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Persistence"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Amplitude"), 1.0).into(),
                IntParam::new(LabelAndHoverText::label_only("Seed"), 0).into(),
            ],
            Self::MultiscaleSphere => vec![
                IntParam::new(LabelAndHoverText::label_only("Octaves"), 0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Max scale"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Persistence"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Inflation"), 1.0).into(),
                FloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0).into(),
                IntParam::new(LabelAndHoverText::label_only("Seed"), 0).into(),
            ],
            Self::Union | Self::Subtraction | Self::Intersection => {
                vec![FloatParam::new(LabelAndHoverText::label_only("Smoothness"), 1.0).into()]
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
                    + vec2(
                        (1.0 + slot as f32) * node_rect.width() / (of as f32 + 1.0),
                        0.0,
                    )
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

impl NodeParam {
    fn show_controls(&mut self, ui: &mut Ui) {
        match self {
            NodeParam::Int(param) => param.show_controls(ui),
            NodeParam::Float(param) => param.show_controls(ui),
        }
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
    fn new(text: LabelAndHoverText, value: u32) -> Self {
        Self { text, value }
    }

    fn show_controls(&mut self, ui: &mut Ui) {
        option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value).speed(1),
        );
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text.label, self.value)
    }
}

impl FloatParam {
    fn new(text: LabelAndHoverText, value: f32) -> Self {
        Self { text, value }
    }

    fn show_controls(&mut self, ui: &mut Ui) {
        option_drag_value(
            ui,
            self.text.clone(),
            DragValue::new(&mut self.value).speed(0.01),
        );
    }

    fn text_to_display(&self) -> String {
        format!("{} = {}", self.text.label, self.value)
    }
}

fn prepare_text(ui: &Ui, text: String, font_id: FontId, color: Color32) -> Arc<Galley> {
    ui.fonts(|f| f.layout_no_wrap(text, font_id, color))
}

fn compute_delta_to_resolve_overlaps(
    node_rects: &BTreeMap<NodeID, Rect>,
    moved_node_id: NodeID,
    moved_node_rect: Rect,
) -> Vec2 {
    const EXPANSION: Vec2 = vec2(MIN_NODE_SEPARATION * 0.5, MIN_NODE_SEPARATION * 0.5);

    let mut moved_node_rect = moved_node_rect.expand2(EXPANSION);
    let mut total_delta = Vec2::ZERO;

    // A few iterations in case we push into someone else
    for _ in 0..6 {
        let mut moved = false;

        for (&node_id, node_rect) in node_rects {
            if node_id == moved_node_id {
                continue;
            }
            let node_rect = node_rect.expand2(EXPANSION);

            if moved_node_rect.intersects(node_rect) {
                // Compute minimal push on x or y to separate
                let dx_left = node_rect.left() - moved_node_rect.right(); // negative
                let dx_right = node_rect.right() - moved_node_rect.left(); // positive
                let dy_up = node_rect.top() - moved_node_rect.bottom(); // negative
                let dy_down = node_rect.bottom() - moved_node_rect.top(); // positive

                // Amount to move moved_node_rect so it *just* stops intersecting
                let push_x = if dx_right.abs() < dx_left.abs() {
                    dx_right
                } else {
                    dx_left
                };
                let push_y = if dy_down.abs() < dy_up.abs() {
                    dy_down
                } else {
                    dy_up
                };

                // Choose smallest magnitude axis (separating axis)
                let (mx, my) = (push_x.abs(), push_y.abs());
                let delta = if mx < my {
                    vec2(push_x, 0.0)
                } else {
                    vec2(0.0, push_y)
                };

                // Apply to the node’s world position
                total_delta += delta;
                moved_node_rect = moved_node_rect.translate(delta);
                moved = true;
            }
        }

        if !moved {
            break;
        }
    }

    total_delta
}
