use super::{MetaNode, MetaNodeData, MetaNodeID, MetaNodeKind, MetaPort};
use crate::editor::{
    MetaGraphStatus, PanZoomState,
    layout::{LayoutScratch, LayoutableGraph, compute_delta_to_resolve_overlaps, layout_vertical},
};
use impact::{
    egui::{
        Color32, Context, CursorIcon, Id, Key, PointerButton, Pos2, Rect, Sense, Stroke, Vec2,
        Window, pos2, vec2,
    },
    impact_containers::KeyIndexMapper,
};
use std::collections::{BTreeMap, BTreeSet};

const CANVAS_DEFAULT_POS: Pos2 = pos2(200.0, 22.0);
const CANVAS_DEFAULT_SIZE: Vec2 = vec2(400.0, 600.0);

const MIN_NODE_SEPARATION: f32 = 8.0;
const NEW_NODE_GAP: f32 = 40.0;

const AUTO_LAYOUT_HORIZONTAL_GAP: f32 = 16.0;
const AUTO_LAYOUT_VERTICAL_GAP: f32 = 40.0;

const EDGE_WIDTH: f32 = 2.0;
const PENDING_EDGE_WIDTH: f32 = 2.0;
const EDGE_COLOR: Color32 = Color32::WHITE;
const PENDING_EDGE_COLOR: Color32 = Color32::LIGHT_GRAY;

const STATUS_DOT_RADIUS: f32 = 6.0;
const STATUS_DOT_OFFSET: Vec2 = vec2(12.0, 12.0);
const STATUS_DOT_VALID_COLOR: Color32 = Color32::GREEN;
const STATUS_DOT_INVALID_COLOR: Color32 = Color32::RED;
const STATUS_DOT_VALID_HOVER_TEXT: &str = "The graph is complete";
const STATUS_DOT_INVALID_HOVER_TEXT: &str = "The graph is not complete";

#[derive(Clone, Debug)]
pub struct MetaGraphCanvas {
    pub pan_zoom_state: PanZoomState,
    pub nodes: BTreeMap<MetaNodeID, MetaNode>,
    pub selected_node_id: Option<MetaNodeID>,
    pub pending_edge: Option<PendingEdge>,
    pub is_panning: bool,
    pub dragging_node_id: Option<MetaNodeID>,
    node_id_counter: MetaNodeID,
}

#[derive(Clone, Debug)]
pub struct MetaCanvasScratch {
    node_rects: BTreeMap<MetaNodeID, Rect>,
    index_map: KeyIndexMapper<MetaNodeID>,
    layout: LayoutScratch,
}

#[derive(Clone, Debug)]
pub struct CanvasShowResult {
    pub connectivity_may_have_changed: bool,
}

#[derive(Clone, Debug)]
pub struct PendingEdge {
    pub from_node: MetaNodeID,
    pub from_port: MetaPort,
}

struct LayoutableMetaGraph<'a> {
    index_map: &'a KeyIndexMapper<MetaNodeID>,
    nodes: &'a BTreeMap<MetaNodeID, MetaNode>,
    rects: &'a mut BTreeMap<MetaNodeID, Rect>,
}

impl MetaGraphCanvas {
    pub fn new() -> Self {
        Self {
            pan_zoom_state: PanZoomState::new(),
            nodes: BTreeMap::new(),
            selected_node_id: None,
            pending_edge: None,
            is_panning: false,
            dragging_node_id: None,
            node_id_counter: 0,
        }
    }

    fn cursor_should_be_hidden(&self) -> bool {
        self.is_panning || self.dragging_node_id.is_some()
    }

    pub fn node(&self, node_id: MetaNodeID) -> &MetaNode {
        self.nodes.get(&node_id).unwrap()
    }

    pub fn node_mut(&mut self, node_id: MetaNodeID) -> &mut MetaNode {
        self.nodes.get_mut(&node_id).unwrap()
    }

    fn get_attached_node_and_port(
        &self,
        node_id: MetaNodeID,
        port: MetaPort,
    ) -> Option<(MetaNodeID, MetaPort)> {
        let node = self.nodes.get(&node_id)?;
        let attached_node_id = node.get_node_attached_to_port(port)?;
        let attached_node = self.nodes.get(&attached_node_id)?;
        let attached_port = attached_node.get_port_node_is_attached_to(node_id, port)?;
        Some((attached_node_id, attached_port))
    }

    fn node_can_reach_other(&self, node_id: MetaNodeID, other_node_id: MetaNodeID) -> bool {
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

    pub fn next_node_id(&mut self) -> MetaNodeID {
        let node_id = self.node_id_counter;
        self.node_id_counter += 1;
        node_id
    }

    pub fn remove_node(&mut self, node_id: MetaNodeID) {
        // Never delete root node
        if self
            .nodes
            .get(&node_id)
            .is_some_and(|node| node.data.kind.is_root())
        {
            return;
        }

        if self.selected_node_id == Some(node_id) {
            self.selected_node_id = None;
        }

        self.detach_parent_of(node_id);

        let n_child_slots = self
            .nodes
            .get(&node_id)
            .map_or(0, |node| node.data.kind.port_config().children);

        for slot in 0..n_child_slots {
            self.detach_child_of(node_id, slot);
        }

        self.nodes.remove(&node_id);

        self.clear_pending_edge_if_from(node_id);
    }

    pub fn change_node_kind(&mut self, node_id: MetaNodeID, new_kind: MetaNodeKind) {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return;
        };
        let n_old_child_slots = node.children.len();
        let n_new_child_slots = new_kind.port_config().children;

        // Detach dropped children and re-attach if there are available slots
        if n_new_child_slots < n_old_child_slots {
            for slot in n_new_child_slots..n_old_child_slots {
                if let Some(child_node_id) = self.detach_child_of(node_id, slot) {
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

    fn clear_pending_edge_if_from(&mut self, node_id: MetaNodeID) {
        if self
            .pending_edge
            .as_ref()
            .is_some_and(|pending_edge| pending_edge.from_node == node_id)
        {
            self.pending_edge = None;
        }
    }

    fn can_attach(
        &self,
        parent_node_id: MetaNodeID,
        child_node_id: MetaNodeID,
        child_slot: usize,
    ) -> bool {
        if parent_node_id == child_node_id {
            return false;
        }

        // If they are already connected, attaching would create a cycle
        if self.node_can_reach_other(child_node_id, parent_node_id) {
            return false;
        }

        // Slot must exist
        if self
            .nodes
            .get(&parent_node_id)
            .and_then(|p| p.children.get(child_slot))
            .is_none()
        {
            return false;
        }

        true
    }

    fn try_attach(
        &mut self,
        parent_node_id: MetaNodeID,
        child_node_id: MetaNodeID,
        child_slot: usize,
    ) -> bool {
        if !self.can_attach(parent_node_id, child_node_id, child_slot) {
            return false;
        }

        self.detach_parent_of(child_node_id);
        if let Some(child_node) = self.nodes.get_mut(&child_node_id) {
            child_node.parent = Some(parent_node_id);
        }

        self.detach_child_of(parent_node_id, child_slot);
        if let Some(parent_node) = self.nodes.get_mut(&parent_node_id) {
            parent_node.children[child_slot] = Some(child_node_id);
        }

        true
    }

    /// Returns the ID of the detached parent node.
    fn detach_parent_of(&mut self, node_id: MetaNodeID) -> Option<MetaNodeID> {
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
    fn detach_child_of(&mut self, node_id: MetaNodeID, slot: usize) -> Option<MetaNodeID> {
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

    pub fn show(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        ctx: &Context,
        graph_status: MetaGraphStatus,
        pending_new_node: Option<(MetaNodeID, MetaNodeData)>,
        perform_layout: bool,
        auto_attach: bool,
    ) -> CanvasShowResult {
        let mut connectivity_may_have_changed = false;

        Window::new("SDF graph")
            .default_pos(CANVAS_DEFAULT_POS)
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
                self.pan_zoom_state.handle_drag(&canvas_response);

                self.pan_zoom_state.handle_scroll(ui, canvas_rect);

                if canvas_response.clicked() {
                    if self.pending_edge.is_some() {
                        self.pending_edge = None;
                    } else if self.selected_node_id.is_some() {
                        self.selected_node_id = None;
                    }
                }

                scratch.node_rects.clear();
                let world_node_rects = &mut scratch.node_rects;

                for (&node_id, node) in &mut self.nodes {
                    node.data.prepare_text(ui, self.pan_zoom_state.zoom);

                    world_node_rects.insert(
                        node_id,
                        Rect::from_min_size(node.position, node.data.compute_size()),
                    );
                }

                // Handle pending new node

                if let Some((node_id, mut data)) = pending_new_node {
                    data.prepare_text(ui, self.pan_zoom_state.zoom);
                    let node_size = data.compute_size();

                    let position = if let Some(selected_node) = self
                        .selected_node_id
                        .and_then(|node_id| self.nodes.get(&node_id))
                    {
                        let selected_node_rect = world_node_rects.values().last().unwrap();
                        selected_node.position
                            + vec2(
                                0.5 * (selected_node_rect.width() - node_size.x),
                                selected_node_rect.height() + NEW_NODE_GAP,
                            )
                    } else if let Some(last_node) = self.nodes.values().last() {
                        let last_node_rect = world_node_rects.values().last().unwrap();
                        last_node.position
                            + vec2(
                                0.5 * (last_node_rect.width() - node_size.x),
                                last_node_rect.height() + NEW_NODE_GAP,
                            )
                    } else {
                        self.pan_zoom_state
                            .screen_pos_to_world_space(canvas_origin, canvas_rect.center_top())
                            + vec2(-0.5 * node_size.x, 0.0)
                    };

                    let mut world_node_rect = Rect::from_min_size(position, node_size);

                    let mut node = MetaNode::new(position, data);

                    let resolve_delta = compute_delta_to_resolve_overlaps(
                        || world_node_rects.iter().map(|(id, rect)| (*id, *rect)),
                        node_id,
                        world_node_rect,
                        MIN_NODE_SEPARATION,
                    );

                    world_node_rect = world_node_rect.translate(resolve_delta);
                    node.position += resolve_delta;

                    self.nodes.insert(node_id, node);
                    world_node_rects.insert(node_id, world_node_rect);

                    if auto_attach
                        && let Some(selected_node_id) = self.selected_node_id
                        && let Some(free_child_slot) = self
                            .nodes
                            .get(&selected_node_id)
                            .and_then(|selected_node| selected_node.first_free_child_slot())
                        && self.try_attach(selected_node_id, node_id, free_child_slot)
                    {
                        connectivity_may_have_changed = true;
                    }

                    self.selected_node_id = Some(node_id);
                }

                if perform_layout {
                    let origin = self
                        .pan_zoom_state
                        .screen_pos_to_world_space(canvas_origin, canvas_rect.center_top());

                    let mut layoutable_graph = LayoutableMetaGraph::new(
                        &mut scratch.index_map,
                        &self.nodes,
                        world_node_rects,
                    );
                    layout_vertical(
                        &mut scratch.layout,
                        &mut layoutable_graph,
                        origin,
                        AUTO_LAYOUT_HORIZONTAL_GAP,
                        AUTO_LAYOUT_VERTICAL_GAP,
                    );

                    for (node, node_rect) in self.nodes.values_mut().zip(world_node_rects.values())
                    {
                        node.position = node_rect.min;
                    }
                }

                for ((&node_id, node), &world_node_rect) in
                    self.nodes.iter_mut().zip(world_node_rects.values())
                {
                    let node_rect = self
                        .pan_zoom_state
                        .world_rect_to_screen_space(canvas_origin, world_node_rect);

                    let node_response = ui.interact(
                        node_rect,
                        Id::new(("meta_node", node_id)),
                        Sense::click_and_drag(),
                    );

                    if node_response.drag_started() {
                        self.dragging_node_id = Some(node_id);
                    }
                    if node_response.drag_stopped() && self.dragging_node_id == Some(node_id) {
                        self.dragging_node_id = None;
                    }

                    // Handle node selection

                    if node_response.clicked() && self.pending_edge.is_none() {
                        self.selected_node_id = Some(node_id);
                    }

                    let is_selected = self.selected_node_id == Some(node_id);

                    // Handle node dragging

                    if node_response.dragged() {
                        let delta = self
                            .pan_zoom_state
                            .screen_vec_to_world_space(node_response.drag_delta());

                        let moved_node_rect = world_node_rect.translate(delta);
                        let resolve_delta = compute_delta_to_resolve_overlaps(
                            || world_node_rects.iter().map(|(id, rect)| (*id, *rect)),
                            node_id,
                            moved_node_rect,
                            MIN_NODE_SEPARATION,
                        );

                        node.position += delta + resolve_delta;
                    }

                    node.data
                        .paint(&painter, node_rect, self.pan_zoom_state.zoom, is_selected);
                }

                // We will only need node rects in screen space from now
                for node_rect in world_node_rects.values_mut() {
                    *node_rect = self
                        .pan_zoom_state
                        .world_rect_to_screen_space(canvas_origin, *node_rect);
                }
                let node_rects = &scratch.node_rects;

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

                        let from = MetaPort::Child {
                            slot,
                            of: parent_node.children.len(),
                        }
                        .center(parent_rect);

                        let to = MetaPort::Parent.center(node_rect);

                        painter.line_segment(
                            [from, to],
                            Stroke {
                                width: EDGE_WIDTH * self.pan_zoom_state.zoom,
                                color: EDGE_COLOR,
                            },
                        );
                    }
                }

                // Draw ports

                for (&node_id, node_rect) in node_rects {
                    for port in self.node(node_id).data.kind.port_config().ports() {
                        let mut enabled = true;
                        let mut highlighted = false;

                        if let Some(pending_edge) = &self.pending_edge {
                            // Ports we can attach the pending edge to are
                            // enabled and highlighted
                            match (pending_edge.from_port, port) {
                                (MetaPort::Parent, MetaPort::Child { slot, .. }) => {
                                    let child_node_id = pending_edge.from_node;
                                    let parent_node_id = node_id;
                                    enabled = self.can_attach(parent_node_id, child_node_id, slot);
                                    highlighted = enabled;
                                }
                                (MetaPort::Child { slot, .. }, MetaPort::Parent) => {
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
                            self.pan_zoom_state.zoom,
                            self.cursor_should_be_hidden(),
                        );

                        if response.clicked() {
                            // Detach if there is a node attached to the port
                            if self.pending_edge.is_none()
                                && self.get_attached_node_and_port(node_id, port).is_some()
                            {
                                match port {
                                    MetaPort::Parent => {
                                        self.detach_parent_of(node_id);
                                    }
                                    MetaPort::Child { slot, .. } => {
                                        self.detach_child_of(node_id, slot);
                                    }
                                }

                                // Create a pending edge from the remaining attached port
                                self.pending_edge = Some(PendingEdge {
                                    from_node: node_id,
                                    from_port: port,
                                });

                                connectivity_may_have_changed = true;

                                continue;
                            }

                            if let Some(pending_edge) = &self.pending_edge {
                                match (pending_edge.from_port, port) {
                                    (MetaPort::Parent, MetaPort::Child { slot, .. }) => {
                                        let child_node_id = pending_edge.from_node;
                                        let parent_node_id = node_id;
                                        if self.try_attach(parent_node_id, child_node_id, slot) {
                                            self.pending_edge = None;
                                            connectivity_may_have_changed = true;
                                        }
                                    }
                                    (MetaPort::Child { slot, .. }, MetaPort::Parent) => {
                                        let parent_node_id = pending_edge.from_node;
                                        let child_node_id = node_id;
                                        if self.try_attach(parent_node_id, child_node_id, slot) {
                                            self.pending_edge = None;
                                            connectivity_may_have_changed = true;
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
                        connectivity_may_have_changed = true;
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
                            width: PENDING_EDGE_WIDTH * self.pan_zoom_state.zoom,
                            color: PENDING_EDGE_COLOR,
                        },
                    );
                }

                // Draw status dot

                let status_dot_pos = canvas_rect.min + STATUS_DOT_OFFSET;

                let status_dot_rect = Rect::from_center_size(
                    status_dot_pos,
                    vec2(2.0 * STATUS_DOT_RADIUS, 2.0 * STATUS_DOT_RADIUS),
                );
                let status_dot_response =
                    ui.interact(status_dot_rect, Id::new("status_dot"), Sense::hover());

                let (status_dot_color, status_dot_text) = match graph_status {
                    MetaGraphStatus::Complete => {
                        (STATUS_DOT_VALID_COLOR, STATUS_DOT_VALID_HOVER_TEXT)
                    }
                    MetaGraphStatus::Incomplete => {
                        (STATUS_DOT_INVALID_COLOR, STATUS_DOT_INVALID_HOVER_TEXT)
                    }
                };

                painter.circle_filled(status_dot_pos, STATUS_DOT_RADIUS, status_dot_color);

                status_dot_response.on_hover_text(status_dot_text);

                // Potentially hide cursor

                if self.cursor_should_be_hidden() {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::None);
                }
            });

        CanvasShowResult {
            connectivity_may_have_changed,
        }
    }
}

impl MetaCanvasScratch {
    pub fn new() -> Self {
        Self {
            node_rects: BTreeMap::new(),
            index_map: KeyIndexMapper::new(),
            layout: LayoutScratch::new(),
        }
    }
}

impl<'a> LayoutableMetaGraph<'a> {
    fn new(
        index_map: &'a mut KeyIndexMapper<MetaNodeID>,
        nodes: &'a BTreeMap<MetaNodeID, MetaNode>,
        rects: &'a mut BTreeMap<MetaNodeID, Rect>,
    ) -> Self {
        index_map.clear();
        index_map.reserve(nodes.len());
        for node_id in nodes.keys() {
            index_map.push_key(*node_id);
        }
        Self {
            index_map: &*index_map,
            nodes,
            rects,
        }
    }
}

impl<'a> LayoutableGraph for LayoutableMetaGraph<'a> {
    fn n_nodes(&self) -> usize {
        self.nodes.len()
    }

    fn child_indices(&self, node_idx: usize) -> impl Iterator<Item = usize> {
        let node_id = self.index_map.key_at_idx(node_idx);
        self.nodes[&node_id]
            .children
            .iter()
            .filter_map(|child_id| child_id.map(|id| self.index_map.idx(id)))
    }

    fn node_rect_mut(&mut self, node_idx: usize) -> &mut Rect {
        let node_id = self.index_map.key_at_idx(node_idx);
        self.rects.get_mut(&node_id).unwrap()
    }
}
