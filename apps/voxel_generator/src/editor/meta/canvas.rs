use super::{
    MetaNode, MetaNodeData, MetaNodeID, MetaNodeKind, MetaNodeLink, MetaPort,
    build::BuildScratch,
    data_type::{DataTypeScratch, update_edge_data_types},
};
use crate::editor::{
    MAX_ZOOM, MIN_ZOOM, MetaGraphStatus, PanZoomState,
    layout::{LayoutScratch, LayoutableGraph, compute_delta_to_resolve_overlaps, layout_vertical},
    meta::{
        MetaPaletteColor, ResolvedMetaPort,
        data_type::EdgeDataType,
        io::{IOMetaNodeGraph, IOMetaNodeGraphRef},
    },
    util::create_bezier_edge,
};
use allocator_api2::{alloc::Allocator, vec::Vec as AVec};
use anyhow::{Context as _, Result};
use impact::{
    egui::{
        Color32, Context, CursorIcon, Id, Key, PointerButton, Pos2, Rect, Sense, Vec2, Window,
        epaint::PathStroke, pos2, vec2,
    },
    impact_containers::{BitVector, KeyIndexMapper},
};
use std::{collections::BTreeMap, path::Path};

const CANVAS_DEFAULT_POS: Pos2 = pos2(200.0, 22.0);
const CANVAS_DEFAULT_SIZE: Vec2 = vec2(400.0, 600.0);

const MIN_NODE_SEPARATION: f32 = 8.0;
const NEW_NODE_GAP: f32 = 40.0;

const AUTO_LAYOUT_HORIZONTAL_GAP: f32 = 32.0;
const AUTO_LAYOUT_VERTICAL_GAP: f32 = 40.0;

const EDGE_WIDTH: f32 = 2.0;
const PENDING_EDGE_WIDTH: f32 = 2.0;

const STATUS_DOT_RADIUS: f32 = 6.0;
const STATUS_DOT_OFFSET: Vec2 = vec2(12.0, 12.0);
const STATUS_DOT_IN_SYNC_COLOR: Color32 = Color32::GREEN;
const STATUS_DOT_DIRTY_COLOR: Color32 = Color32::YELLOW;
const STATUS_DOT_INVALID_COLOR: Color32 = Color32::RED;
const STATUS_DOT_IN_SYNC_HOVER_TEXT: &str = "The graph is in sync";
const STATUS_DOT_DIRTY_HOVER_TEXT: &str = "The graph is out of sync";
const STATUS_DOT_INVALID_HOVER_TEXT: &str = "The graph is not valid";

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
    search: SearchScratch,
    data_type: DataTypeScratch,
    layout: LayoutScratch,
    pub build: BuildScratch,
}

#[derive(Clone, Debug)]
struct SearchScratch {
    stack: Vec<MetaNodeID>,
    seen: BitVector,
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
            pan_zoom_state: PanZoomState::default(),
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

    fn node_has_link_at_port(&self, node_id: MetaNodeID, port: MetaPort) -> bool {
        let Some(node) = self.nodes.get(&node_id) else {
            return false;
        };
        match port {
            MetaPort::Parent { slot, .. } => node.links_to_parents.get(slot).is_some(),
            MetaPort::Child { slot, .. } => node.links_to_children.get(slot).is_some(),
        }
    }

    fn node_can_reach_other(
        &self,
        scratch: &mut SearchScratch,
        node_id: MetaNodeID,
        other_node_id: MetaNodeID,
    ) -> bool {
        let stack = &mut scratch.stack;
        let seen = &mut scratch.seen;

        stack.clear();
        stack.push(node_id);

        seen.resize_and_unset_all(self.node_id_counter as usize);

        while let Some(node_id) = stack.pop() {
            if seen.set_bit(node_id as usize) {
                continue;
            }
            if node_id == other_node_id {
                return true;
            }
            if let Some(node) = self.nodes.get(&node_id) {
                for child_node_id in node
                    .links_to_children
                    .iter()
                    .filter_map(|link| link.map(|link| link.to_node))
                {
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
            .is_some_and(|node| node.data.kind.is_output())
        {
            return;
        }

        if self.selected_node_id == Some(node_id) {
            self.selected_node_id = None;
        }

        let (n_parent_slots, n_child_slots) = self.nodes.get(&node_id).map_or((0, 0), |node| {
            (node.links_to_parents.len(), node.links_to_children.len())
        });

        for slot in 0..n_parent_slots {
            self.detach_parent_of(node_id, slot);
        }

        for slot in 0..n_child_slots {
            self.detach_child_of(node_id, slot);
        }

        self.nodes.remove(&node_id);

        self.clear_pending_edge_if_from(node_id);
    }

    pub fn change_node_kind(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        node_id: MetaNodeID,
        new_kind: MetaNodeKind,
    ) {
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };

        let n_parent_slots = node.links_to_parents.len();
        let n_old_child_slots = node.links_to_children.len();
        let n_new_child_slots = new_kind.n_child_slots();

        if n_new_child_slots < n_old_child_slots {
            for slot in n_new_child_slots..n_old_child_slots {
                self.detach_child_of(node_id, slot);
            }
        }

        self.node_mut(node_id).change_kind(new_kind);

        // Since we have changed the kind, some existing links may no longer be
        // valid. But we need to update the data types based on the new node
        // kind before we can check for and remove invalid links.
        self.update_edge_data_types(scratch);

        // Detach all children and parents with which the ports have become
        // incompatible
        for slot in 0..n_new_child_slots {
            if self.link_to_child_exists_with_invalid_data_type(node_id, slot) {
                self.detach_child_of(node_id, slot);
            }
        }
        for slot in 0..n_parent_slots {
            if self.link_to_parent_exists_with_invalid_data_type(node_id, slot) {
                self.detach_parent_of(node_id, slot);
            }
        }
    }

    pub fn change_parent_port_count(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        node_id: MetaNodeID,
        new_count: usize,
    ) {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return;
        };

        let old_count = node.links_to_parents.len();

        if new_count > old_count {
            node.links_to_parents.resize(new_count, None);
        } else if new_count < old_count {
            for slot in new_count..old_count {
                if let Some(MetaNodeLink {
                    to_node: parent_node_id,
                    to_slot: child_slot_on_parent,
                }) = self.detach_parent_of(node_id, slot)
                {
                    for slot in 0..new_count {
                        if self.node(node_id).links_to_parents[slot].is_none() {
                            self.try_attach(
                                &mut scratch.search,
                                parent_node_id,
                                child_slot_on_parent,
                                node_id,
                                slot,
                            );
                        }
                    }
                };
            }
            self.node_mut(node_id).links_to_parents.truncate(new_count);
        }
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
        search_scratch: &mut SearchScratch,
        parent_node_id: MetaNodeID,
        child_slot_on_parent: usize,
        child_node_id: MetaNodeID,
        parent_slot_on_child: usize,
    ) -> bool {
        if parent_node_id == child_node_id {
            return false;
        }

        // If they are already connected, attaching would create a cycle
        if self.node_can_reach_other(search_scratch, child_node_id, parent_node_id) {
            return false;
        }

        // Nodes must exist
        let Some(parent_node) = self.nodes.get(&parent_node_id) else {
            return false;
        };
        let Some(child_node) = self.nodes.get(&child_node_id) else {
            return false;
        };

        // Slots must exist
        if child_slot_on_parent >= parent_node.links_to_children.len() {
            return false;
        }
        if parent_slot_on_child >= child_node.links_to_parents.len() {
            return false;
        }

        // Output and input data types must be compatible
        EdgeDataType::connection_allowed(
            parent_node.input_data_types[child_slot_on_parent],
            child_node.output_data_type,
        )
    }

    fn try_attach(
        &mut self,
        search_scratch: &mut SearchScratch,
        parent_node_id: MetaNodeID,
        child_slot_on_parent: usize,
        child_node_id: MetaNodeID,
        parent_slot_on_child: usize,
    ) -> bool {
        if !self.can_attach(
            search_scratch,
            parent_node_id,
            child_slot_on_parent,
            child_node_id,
            parent_slot_on_child,
        ) {
            return false;
        }

        self.detach_parent_of(child_node_id, parent_slot_on_child);
        if let Some(child_node) = self.nodes.get_mut(&child_node_id) {
            child_node.links_to_parents[parent_slot_on_child] = Some(MetaNodeLink {
                to_node: parent_node_id,
                to_slot: child_slot_on_parent,
            });
        }

        self.detach_child_of(parent_node_id, child_slot_on_parent);
        if let Some(parent_node) = self.nodes.get_mut(&parent_node_id) {
            parent_node.links_to_children[child_slot_on_parent] = Some(MetaNodeLink {
                to_node: child_node_id,
                to_slot: parent_slot_on_child,
            });
        }

        true
    }

    /// Returns the ID of the detached parent node and the slot on the parent
    /// node the child was attached to.
    fn detach_parent_of(
        &mut self,
        child_node_id: MetaNodeID,
        parent_slot_on_child: usize,
    ) -> Option<MetaNodeLink> {
        let child_node = self.nodes.get_mut(&child_node_id)?;

        let child_link_to_parent = child_node
            .links_to_parents
            .get_mut(parent_slot_on_child)
            .and_then(|link| link.take())?;

        if let Some(parent_link_to_child) = self
            .nodes
            .get_mut(&child_link_to_parent.to_node)
            .and_then(|parent_node| {
                parent_node
                    .links_to_children
                    .get_mut(child_link_to_parent.to_slot)
            })
        {
            *parent_link_to_child = None;
        }

        Some(child_link_to_parent)
    }

    /// Returns the ID of the detached child node and the slot on the child node
    /// the parent was attached to.
    fn detach_child_of(
        &mut self,
        parent_node_id: MetaNodeID,
        child_slot_on_parent: usize,
    ) -> Option<MetaNodeLink> {
        let parent_node = self.nodes.get_mut(&parent_node_id)?;

        let parent_link_to_child = parent_node
            .links_to_children
            .get_mut(child_slot_on_parent)
            .and_then(|link| link.take())?;

        if let Some(child_link_to_parent) = self
            .nodes
            .get_mut(&parent_link_to_child.to_node)
            .and_then(|child_node| {
                child_node
                    .links_to_parents
                    .get_mut(parent_link_to_child.to_slot)
            })
        {
            *child_link_to_parent = None;
        }

        Some(parent_link_to_child)
    }

    fn link_to_parent_exists_with_invalid_data_type(
        &mut self,
        child_node_id: MetaNodeID,
        parent_slot_on_child: usize,
    ) -> bool {
        let Some(child_node) = self.nodes.get(&child_node_id) else {
            return false;
        };

        let Some(child_link_to_parent) = child_node
            .links_to_parents
            .get(parent_slot_on_child)
            .and_then(|link| link.as_ref())
        else {
            return false;
        };

        let Some(parent_node) = self.nodes.get(&child_link_to_parent.to_node) else {
            return false;
        };

        !EdgeDataType::connection_allowed(
            parent_node.input_data_types[child_link_to_parent.to_slot],
            child_node.output_data_type,
        )
    }

    fn link_to_child_exists_with_invalid_data_type(
        &mut self,
        parent_node_id: MetaNodeID,
        child_slot_on_parent: usize,
    ) -> bool {
        let Some(parent_node) = self.nodes.get(&parent_node_id) else {
            return false;
        };

        let Some(parent_link_to_child) = parent_node
            .links_to_children
            .get(child_slot_on_parent)
            .and_then(|link| link.as_ref())
        else {
            return false;
        };

        let Some(child_node) = self.nodes.get(&parent_link_to_child.to_node) else {
            return false;
        };

        !EdgeDataType::connection_allowed(
            parent_node.input_data_types[child_slot_on_parent],
            child_node.output_data_type,
        )
    }

    pub fn show(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        ctx: &Context,
        graph_status: MetaGraphStatus,
        pending_new_node: Option<(MetaNodeID, MetaNodeData)>,
        mut perform_layout: bool,
        auto_attach: bool,
        auto_layout: bool,
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

                self.pan_zoom_state
                    .handle_drag(ui, canvas_rect, &mut self.is_panning);

                self.pan_zoom_state.handle_scroll(ui, canvas_rect);

                if canvas_response.clicked() {
                    if self.pending_edge.is_some() {
                        self.pending_edge = None;
                    } else if self.selected_node_id.is_some() {
                        self.selected_node_id = None;
                    }
                }

                // Handle cancellation of pending edge or node deletion with keyboard

                if ui.input(|i| i.key_pressed(Key::Delete)) {
                    if self.pending_edge.is_some() {
                        self.pending_edge = None;
                    } else if let Some(selected_id) = self.selected_node_id {
                        self.remove_node(selected_id);
                        if auto_layout {
                            perform_layout = true;
                        }
                        connectivity_may_have_changed = true;
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

                    let output_data_type = node.output_data_type;
                    let is_leaf = node.data.kind.n_child_slots() == 0;

                    self.nodes.insert(node_id, node);
                    world_node_rects.insert(node_id, world_node_rect);

                    if auto_attach
                        && let Some(selected_node_id) = self.selected_node_id
                        && let Some(free_child_slot) =
                            self.nodes.get(&selected_node_id).and_then(|selected_node| {
                                selected_node.first_free_child_slot_accepting_type(output_data_type)
                            })
                        && self.try_attach(
                            &mut scratch.search,
                            selected_node_id,
                            free_child_slot,
                            node_id,
                            0,
                        )
                    {
                        connectivity_may_have_changed = true;
                    }

                    if !is_leaf || self.selected_node_id.is_none() {
                        self.selected_node_id = Some(node_id);
                    }
                }

                if perform_layout {
                    let origin = world_node_rects
                        .get(&0)
                        .map_or_else(Pos2::default, Rect::center_top);

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

                    if self.is_panning {
                        self.dragging_node_id = None;
                    } else {
                        let node_response = ui.interact(
                            node_rect,
                            Id::new(("meta_node", node_id)),
                            Sense::click_and_drag(),
                        );

                        // Handle node selection

                        if node_response.clicked() && self.pending_edge.is_none() {
                            self.selected_node_id = Some(node_id);
                        }

                        // Handle node dragging

                        if node_response.drag_started_by(PointerButton::Primary) {
                            self.dragging_node_id = Some(node_id);
                        }
                        if node_response.drag_stopped_by(PointerButton::Primary)
                            && self.dragging_node_id == Some(node_id)
                        {
                            self.dragging_node_id = None;
                        }

                        if node_response.dragged_by(PointerButton::Primary) {
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
                    }

                    let is_selected = self.selected_node_id == Some(node_id);

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

                for (&child_node_id, child_node) in &self.nodes {
                    let child_node_rect = &node_rects[&child_node_id];

                    for (parent_slot_on_child, child_link_to_parent) in
                        child_node.links_to_parents.iter().enumerate()
                    {
                        let &Some(MetaNodeLink {
                            to_node: parent_node_id,
                            to_slot: child_slot_on_parent,
                        }) = child_link_to_parent
                        else {
                            continue;
                        };
                        let Some(parent_node) = self.nodes.get(&parent_node_id) else {
                            continue;
                        };

                        let input_data_type = parent_node.input_data_types[child_slot_on_parent];
                        let output_data_type = child_node.output_data_type;
                        let edge_color = if EdgeDataType::connection_allowed(
                            input_data_type,
                            output_data_type,
                        ) {
                            input_data_type.color().standard
                        } else {
                            MetaPaletteColor::red().standard
                        };

                        let parent_rect = &node_rects[&parent_node_id];

                        let child_pos = MetaPort::child_center(
                            parent_rect,
                            child_slot_on_parent,
                            parent_node.links_to_children.len(),
                        );

                        let parent_pos = MetaPort::parent_center(
                            child_node_rect,
                            parent_slot_on_child,
                            child_node.links_to_parents.len(),
                        );

                        let edge_shape = create_bezier_edge(
                            child_pos,
                            parent_pos,
                            PathStroke::new(EDGE_WIDTH * self.pan_zoom_state.zoom, edge_color),
                        );
                        painter.add(edge_shape);
                    }
                }

                // Draw ports

                let mut pending_edge_port_color = Color32::BLACK;

                for (&node_id, node_rect) in node_rects {
                    let node = self.node(node_id);
                    for ResolvedMetaPort { port, data_type } in node.resolved_ports() {
                        let mut enabled = true;

                        if let Some(pending_edge) = &self.pending_edge {
                            // Ports we can attach the pending edge to are
                            // enabled and highlighted
                            match (pending_edge.from_port, port) {
                                (
                                    MetaPort::Parent {
                                        slot: parent_slot_on_child,
                                        ..
                                    },
                                    MetaPort::Child {
                                        slot: child_slot_on_parent,
                                        ..
                                    },
                                ) => {
                                    let child_node_id = pending_edge.from_node;
                                    let parent_node_id = node_id;
                                    enabled = self.can_attach(
                                        &mut scratch.search,
                                        parent_node_id,
                                        child_slot_on_parent,
                                        child_node_id,
                                        parent_slot_on_child,
                                    );
                                }
                                (
                                    MetaPort::Child {
                                        slot: child_slot_on_parent,
                                        ..
                                    },
                                    MetaPort::Parent {
                                        slot: parent_slot_on_child,
                                        ..
                                    },
                                ) => {
                                    let parent_node_id = pending_edge.from_node;
                                    let child_node_id = node_id;
                                    enabled = self.can_attach(
                                        &mut scratch.search,
                                        parent_node_id,
                                        child_slot_on_parent,
                                        child_node_id,
                                        parent_slot_on_child,
                                    );
                                }
                                _ => {
                                    // Mismatched ports are disabled
                                    enabled = false;
                                }
                            }
                        }

                        let pending_edge_is_from_this_port =
                            self.pending_edge.as_ref().is_some_and(|edge| {
                                edge.from_node == node_id && edge.from_port == port
                            });

                        let port_shape = data_type.port_shape();

                        let port_color = if enabled || pending_edge_is_from_this_port {
                            data_type.color().standard
                        } else {
                            data_type.color().darker
                        };

                        let port_label = data_type.port_label();

                        if pending_edge_is_from_this_port {
                            pending_edge_port_color = port_color;
                        }

                        let response = port.show(
                            ui,
                            &painter,
                            node_id,
                            node_rect,
                            enabled,
                            self.pan_zoom_state.zoom,
                            self.cursor_should_be_hidden(),
                            port_shape,
                            port_color,
                            port_label,
                        );

                        if response.clicked() {
                            // Detach if there is a node attached to the port
                            if self.pending_edge.is_none()
                                && self.node_has_link_at_port(node_id, port)
                            {
                                match port {
                                    MetaPort::Parent { slot, .. } => {
                                        self.detach_parent_of(node_id, slot);
                                    }
                                    MetaPort::Child { slot, .. } => {
                                        self.detach_child_of(node_id, slot);
                                    }
                                }

                                // Create a pending edge from the clicked port
                                self.pending_edge = Some(PendingEdge {
                                    from_node: node_id,
                                    from_port: port,
                                });

                                connectivity_may_have_changed = true;

                                continue;
                            }

                            if let Some(pending_edge) = &self.pending_edge {
                                match (pending_edge.from_port, port) {
                                    (
                                        MetaPort::Parent {
                                            slot: parent_slot_on_child,
                                            ..
                                        },
                                        MetaPort::Child {
                                            slot: child_slot_on_parent,
                                            ..
                                        },
                                    ) => {
                                        let child_node_id = pending_edge.from_node;
                                        let parent_node_id = node_id;
                                        if self.try_attach(
                                            &mut scratch.search,
                                            parent_node_id,
                                            child_slot_on_parent,
                                            child_node_id,
                                            parent_slot_on_child,
                                        ) {
                                            self.pending_edge = None;
                                            connectivity_may_have_changed = true;
                                        }
                                    }
                                    (
                                        MetaPort::Child {
                                            slot: child_slot_on_parent,
                                            ..
                                        },
                                        MetaPort::Parent {
                                            slot: parent_slot_on_child,
                                            ..
                                        },
                                    ) => {
                                        let parent_node_id = pending_edge.from_node;
                                        let child_node_id = node_id;
                                        if self.try_attach(
                                            &mut scratch.search,
                                            parent_node_id,
                                            child_slot_on_parent,
                                            child_node_id,
                                            parent_slot_on_child,
                                        ) {
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

                // Draw pending edge

                if let Some(pending_edge) = &self.pending_edge
                    && let (Some(node_rect), Some(mouse_pos)) = (
                        node_rects.get(&pending_edge.from_node),
                        ui.input(|i| i.pointer.hover_pos()),
                    )
                {
                    let from = pending_edge.from_port.center(node_rect);
                    let to = mouse_pos;
                    let (child_pos, parent_pos) =
                        if let MetaPort::Parent { .. } = pending_edge.from_port {
                            (to, from)
                        } else {
                            (from, to)
                        };
                    let edge_shape = create_bezier_edge(
                        child_pos,
                        parent_pos,
                        PathStroke::new(
                            PENDING_EDGE_WIDTH * self.pan_zoom_state.zoom,
                            pending_edge_port_color,
                        ),
                    );
                    painter.add(edge_shape);
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
                    MetaGraphStatus::InSync => {
                        (STATUS_DOT_IN_SYNC_COLOR, STATUS_DOT_IN_SYNC_HOVER_TEXT)
                    }
                    MetaGraphStatus::Dirty => (STATUS_DOT_DIRTY_COLOR, STATUS_DOT_DIRTY_HOVER_TEXT),
                    MetaGraphStatus::Invalid => {
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

    pub fn update_edge_data_types(&mut self, scratch: &mut MetaCanvasScratch) {
        update_edge_data_types(&mut scratch.data_type, &mut self.nodes);
    }

    pub fn save_graph<A: Allocator>(&self, arena: A, output_path: &Path) -> Result<()> {
        let mut nodes = AVec::with_capacity_in(self.nodes.len(), arena);
        nodes.extend(self.nodes.iter().map(Into::into));

        let graph = IOMetaNodeGraphRef {
            pan: self.pan_zoom_state.pan.into(),
            zoom: self.pan_zoom_state.zoom,
            nodes: nodes.as_slice(),
        };

        impact_io::write_ron_file(&graph, output_path)
    }

    pub fn load_graph(&mut self, scratch: &mut MetaCanvasScratch, path: &Path) -> Result<()> {
        let graph: IOMetaNodeGraph =
            impact_io::parse_ron_file(path).context("Failed to parse graph file")?;

        let mut nodes = BTreeMap::new();
        let mut node_id_counter = 0;

        for io_node in graph.nodes {
            let id = io_node.id;
            let node = io_node
                .try_into()
                .with_context(|| format!("Invalid node in graph file (node ID {id})"))?;
            nodes.insert(id, node);
            node_id_counter = node_id_counter.max(id + 1);
        }

        self.nodes = nodes;
        self.node_id_counter = node_id_counter;

        self.pan_zoom_state =
            PanZoomState::new(graph.pan.into(), graph.zoom.clamp(MIN_ZOOM, MAX_ZOOM));

        self.selected_node_id = None;
        self.pending_edge = None;
        self.is_panning = false;
        self.dragging_node_id = None;

        self.update_edge_data_types(scratch);

        Ok(())
    }
}

impl MetaCanvasScratch {
    pub fn new() -> Self {
        Self {
            node_rects: BTreeMap::new(),
            index_map: KeyIndexMapper::new(),
            search: SearchScratch::new(),
            data_type: DataTypeScratch::new(),
            layout: LayoutScratch::new(),
            build: BuildScratch::new(),
        }
    }
}

impl SearchScratch {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            seen: BitVector::new(),
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
            .links_to_children
            .iter()
            .filter_map(|link| link.map(|link| self.index_map.idx(link.to_node)))
    }

    fn node_rect_mut(&mut self, node_idx: usize) -> &mut Rect {
        let node_id = self.index_map.key_at_idx(node_idx);
        self.rects.get_mut(&node_id).unwrap()
    }
}
