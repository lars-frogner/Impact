use super::{
    MetaNode, MetaNodeData, MetaNodeID, MetaNodeKind, MetaNodeLink, MetaPort,
    build::BuildScratch,
    data_type::{DataTypeScratch, update_edge_data_types},
    show_port,
};
use crate::editor::{
    MAX_ZOOM, MIN_ZOOM, MetaGraphStatus, PanZoomState,
    layout::{LayoutScratch, LayoutableGraph, compute_delta_to_resolve_overlaps, layout_vertical},
    meta::{
        CollapsedMetaSubtree, CollapsedMetaSubtreeChildPort, CollapsedMetaSubtreeParentPort,
        MetaPaletteColor,
        data_type::EdgeDataType,
        io::{IOMetaGraph, IOMetaGraphKind, IOMetaGraphRef},
    },
    util::create_bezier_edge,
};
use allocator_api2::{alloc::Allocator, vec::Vec as AVec};
use anyhow::{Context as _, Result, bail};
use bitflags::bitflags;
use impact::{
    egui::{
        Align, Color32, Context, CursorIcon, Direction, Id, Key, Label, Painter, PointerButton,
        Pos2, Rect, Sense, Ui, Vec2, Window, epaint::PathStroke, pos2, vec2,
    },
    impact_containers::{BitVector, HashMap, HashSet, KeyIndexMapper},
};
use std::{collections::BTreeMap, path::Path};

const CANVAS_DEFAULT_POS: Pos2 = pos2(250.0, 25.0);
const CANVAS_DEFAULT_SIZE: Vec2 = vec2(600.0, 700.0);

const MIN_NODE_SEPARATION: f32 = 8.0;
const NEW_NODE_GAP: f32 = 40.0;
const OUTPUT_NODE_TOP_CLEARING: f32 = 10.0;

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

const MIN_COLLAPSED_PROXY_NODE_SIZE: Vec2 = vec2(80.0, 0.0);

#[derive(Clone, Debug)]
pub struct MetaGraphCanvas {
    pub pan_zoom_state: PanZoomState,
    pub nodes: BTreeMap<MetaNodeID, MetaNode>,
    pub collapsed_nodes: HashSet<MetaNodeID>,
    pub selected_node_id: Option<MetaNodeID>,
    pub pending_edge: Option<PendingEdge>,
    pub is_panning: bool,
    pub dragging_node: Option<DraggingNode>,
    collapse_index: CollapseIndex,
    node_id_counter: MetaNodeID,
}

#[derive(Clone, Debug)]
pub struct MetaCanvasScratch {
    world_node_rects: BTreeMap<MetaNodeID, Rect>,
    screen_node_rects: BTreeMap<MetaNodeID, Rect>,
    subtree_node_ids: Vec<MetaNodeID>,
    layout_lut: MetaLayoutLookupTable,
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

bitflags! {
    pub struct MetaGraphChanges: u16 {
        const NODE_ADDED                = 1 << 0;
        const NODE_REMOVED              = 1 << 1;
        const NODE_ATTACHED             = 1 << 2;
        const NODE_DETACHED             = 1 << 3;
        const PARAMS_CHANGED            = 1 << 4;
        const NAME_CHANGED              = 1 << 5;
        const KIND_CHANGED              = 1 << 6;
        const PARENT_PORT_COUNT_CHANGED = 1 << 7;
        const COLLAPSED_STATE_CHANGED   = 1 << 8;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PendingEdge {
    pub from_node: MetaNodeID,
    pub from_port: MetaPort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraggingNode {
    pub node_id: MetaNodeID,
    pub by_button: PointerButton,
}

#[derive(Debug, Clone, Default)]
pub struct PendingNodeOperations {
    pub addition: Option<PendingNodeAddition>,
    pub removal: Option<PendingNodeRemoval>,
    pub name_update: Option<PendingNodeNameUpdate>,
    pub kind_change: Option<PendingNodeKindChange>,
    pub parent_port_count_change: Option<PendingNodeParentPortCountChange>,
    pub collapsed_state_change: Option<PendingNodeCollapsedStateChange>,
}

#[derive(Debug, Clone)]
pub struct PendingNodeAddition {
    pub node_id: MetaNodeID,
    pub data: MetaNodeData,
}

#[derive(Debug, Clone)]
pub struct PendingNodeRemoval {
    pub node_id: MetaNodeID,
}

#[derive(Debug, Clone)]
pub struct PendingNodeNameUpdate {
    pub node_id: MetaNodeID,
}

#[derive(Debug, Clone)]
pub struct PendingNodeKindChange {
    pub node_id: MetaNodeID,
    pub kind: MetaNodeKind,
}

#[derive(Debug, Clone)]
pub struct PendingNodeParentPortCountChange {
    pub node_id: MetaNodeID,
    pub parent_port_count: usize,
}

#[derive(Debug, Clone)]
pub struct PendingNodeCollapsedStateChange {
    pub node_id: MetaNodeID,
    pub collapsed: bool,
}

#[derive(Clone, Debug)]
struct CollapseIndex {
    visible_subtree_roots: HashSet<MetaNodeID>,
    subtrees_by_root: HashMap<MetaNodeID, CollapsedMetaSubtree>,
    member_to_root: HashMap<MetaNodeID, MetaNodeID>,
}

#[derive(Debug)]
struct LayoutableMetaGraph<'a> {
    lut: &'a MetaLayoutLookupTable,
    visible_node_rects: &'a mut BTreeMap<MetaNodeID, Rect>,
}

#[derive(Clone, Debug)]
struct MetaLayoutLookupTable {
    visible_node_index_map: KeyIndexMapper<MetaNodeID>,
    child_idx_offsets_and_counts: Vec<(usize, usize)>,
    all_child_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
struct AutoAttachInfo {
    attach_as_child: bool,
    parent_node_id: MetaNodeID,
    child_slot_on_parent: usize,
    child_node_id: MetaNodeID,
    parent_slot_on_child: usize,
}

impl MetaGraphCanvas {
    pub fn new() -> Self {
        Self {
            pan_zoom_state: PanZoomState::default(),
            nodes: BTreeMap::new(),
            collapsed_nodes: HashSet::default(),
            selected_node_id: None,
            pending_edge: None,
            is_panning: false,
            dragging_node: None,
            collapse_index: CollapseIndex::new(),
            node_id_counter: 0,
        }
    }

    pub fn reset(&mut self) {
        self.pan_zoom_state = PanZoomState::default();
        self.nodes.clear();
        self.collapsed_nodes.clear();
        self.selected_node_id = None;
        self.pending_edge = None;
        self.is_panning = false;
        self.dragging_node = None;
        self.collapse_index.reset();
        self.node_id_counter = 0;
    }

    fn cursor_should_be_hidden(&self) -> bool {
        self.is_panning || self.dragging_node.is_some()
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

    pub fn next_node_id(&mut self) -> MetaNodeID {
        let node_id = self.node_id_counter;
        self.node_id_counter += 1;
        node_id
    }

    fn remove_node(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        node_id: MetaNodeID,
        changes: &mut MetaGraphChanges,
    ) {
        if self
            .collapse_index
            .node_is_visible_collapsed_subtree_root(node_id)
        {
            obtain_subtree(
                &mut scratch.search,
                &self.nodes,
                self.node_id_counter,
                &mut scratch.subtree_node_ids,
                node_id,
            );
            for &node_id_to_remove in &scratch.subtree_node_ids {
                self.remove_single_node(node_id_to_remove, changes);
            }
        } else {
            self.remove_single_node(node_id, changes);
        }
    }

    fn remove_single_node(&mut self, node_id: MetaNodeID, changes: &mut MetaGraphChanges) {
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
            self.detach_parent_of(node_id, slot, changes);
        }

        for slot in 0..n_child_slots {
            self.detach_child_of(node_id, slot, changes);
        }

        self.nodes.remove(&node_id);
        changes.insert(MetaGraphChanges::NODE_REMOVED);

        if self.collapsed_nodes.remove(&node_id) {
            changes.insert(MetaGraphChanges::COLLAPSED_STATE_CHANGED);
        }
    }

    pub fn change_node_kind(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        node_id: MetaNodeID,
        new_kind: MetaNodeKind,
        changes: &mut MetaGraphChanges,
    ) -> bool {
        let Some(node) = self.nodes.get(&node_id) else {
            return false;
        };

        let n_parent_slots = node.links_to_parents.len();
        let n_old_child_slots = node.links_to_children.len();
        let n_new_child_slots = new_kind.n_child_slots();

        if n_new_child_slots < n_old_child_slots {
            for slot in n_new_child_slots..n_old_child_slots {
                self.detach_child_of(node_id, slot, changes);
            }
        }

        self.node_mut(node_id).change_kind(new_kind);
        changes.insert(MetaGraphChanges::KIND_CHANGED);

        // Since we have changed the kind, some existing links may no longer be
        // valid. But we need to update the data types based on the new node
        // kind before we can check for and remove invalid links.
        self.update_edge_data_types(scratch);

        // Detach all children and parents with which the ports have become
        // incompatible
        for slot in 0..n_new_child_slots {
            if self.link_to_child_exists_with_invalid_data_type(node_id, slot) {
                self.detach_child_of(node_id, slot, changes);
            }
        }
        for slot in 0..n_parent_slots {
            if self.link_to_parent_exists_with_invalid_data_type(node_id, slot) {
                self.detach_parent_of(node_id, slot, changes);
            }
        }

        true
    }

    pub fn change_parent_port_count(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        node_id: MetaNodeID,
        new_count: usize,
        changes: &mut MetaGraphChanges,
    ) {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return;
        };

        let old_count = node.links_to_parents.len();

        if new_count > old_count {
            node.links_to_parents.resize(new_count, None);
            changes.insert(MetaGraphChanges::PARENT_PORT_COUNT_CHANGED);
        } else if new_count < old_count {
            for slot in new_count..old_count {
                if let Some(MetaNodeLink {
                    to_node: parent_node_id,
                    to_slot: child_slot_on_parent,
                }) = self.detach_parent_of(node_id, slot, changes)
                {
                    for slot in 0..new_count {
                        if self.node(node_id).links_to_parents[slot].is_none() {
                            self.try_attach(
                                &mut scratch.search,
                                parent_node_id,
                                child_slot_on_parent,
                                node_id,
                                slot,
                                changes,
                            );
                        }
                    }
                };
            }
            self.node_mut(node_id).links_to_parents.truncate(new_count);
            changes.insert(MetaGraphChanges::PARENT_PORT_COUNT_CHANGED);
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
        if node_can_reach_other(
            search_scratch,
            &self.nodes,
            self.node_id_counter,
            child_node_id,
            parent_node_id,
        ) {
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
        changes: &mut MetaGraphChanges,
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

        self.detach_parent_of(child_node_id, parent_slot_on_child, changes);
        if let Some(child_node) = self.nodes.get_mut(&child_node_id) {
            child_node.links_to_parents[parent_slot_on_child] = Some(MetaNodeLink {
                to_node: parent_node_id,
                to_slot: child_slot_on_parent,
            });
            changes.insert(MetaGraphChanges::NODE_ATTACHED);
        }

        self.detach_child_of(parent_node_id, child_slot_on_parent, changes);
        if let Some(parent_node) = self.nodes.get_mut(&parent_node_id) {
            parent_node.links_to_children[child_slot_on_parent] = Some(MetaNodeLink {
                to_node: child_node_id,
                to_slot: parent_slot_on_child,
            });
            changes.insert(MetaGraphChanges::NODE_ATTACHED);
        }

        true
    }

    /// Returns the ID of the detached parent node and the slot on the parent
    /// node the child was attached to.
    fn detach_parent_of(
        &mut self,
        child_node_id: MetaNodeID,
        parent_slot_on_child: usize,
        changes: &mut MetaGraphChanges,
    ) -> Option<MetaNodeLink> {
        let child_node = self.nodes.get_mut(&child_node_id)?;

        let child_link_to_parent = child_node
            .links_to_parents
            .get_mut(parent_slot_on_child)
            .and_then(|link| link.take())?;

        changes.insert(MetaGraphChanges::NODE_DETACHED);

        if self.pending_edge.is_some_and(|edge| {
            matches!(
                edge.from_port,
                MetaPort::Parent { slot,.. } if slot == parent_slot_on_child
                    && edge.from_node == child_node_id
            ) || matches!(
                edge.from_port,
                MetaPort::Child { slot,.. } if slot == child_link_to_parent.to_slot
                    && edge.from_node == child_link_to_parent.to_node
            )
        }) {
            self.pending_edge = None;
        }

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
            changes.insert(MetaGraphChanges::NODE_DETACHED);
        }

        Some(child_link_to_parent)
    }

    /// Returns the ID of the detached child node and the slot on the child node
    /// the parent was attached to.
    fn detach_child_of(
        &mut self,
        parent_node_id: MetaNodeID,
        child_slot_on_parent: usize,
        changes: &mut MetaGraphChanges,
    ) -> Option<MetaNodeLink> {
        let parent_node = self.nodes.get_mut(&parent_node_id)?;

        let parent_link_to_child = parent_node
            .links_to_children
            .get_mut(child_slot_on_parent)
            .and_then(|link| link.take())?;

        changes.insert(MetaGraphChanges::NODE_DETACHED);

        if self.pending_edge.is_some_and(|edge| {
            matches!(
                edge.from_port,
                MetaPort::Parent { slot,.. } if slot == parent_link_to_child.to_slot
                    && edge.from_node == parent_link_to_child.to_node
            ) || matches!(
                edge.from_port,
                MetaPort::Child { slot,.. } if slot == child_slot_on_parent
                    && edge.from_node == parent_node_id
            )
        }) {
            self.pending_edge = None;
        }

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
            changes.insert(MetaGraphChanges::NODE_DETACHED);
        }

        Some(parent_link_to_child)
    }

    pub fn node_is_collapsed_root(&self, node_id: MetaNodeID) -> bool {
        self.collapsed_nodes.contains(&node_id)
    }

    pub fn set_node_collapsed(
        &mut self,
        node_id: MetaNodeID,
        collapsed: bool,
        changes: &mut MetaGraphChanges,
    ) {
        if collapsed {
            if self.collapsed_nodes.insert(node_id) {
                changes.insert(MetaGraphChanges::COLLAPSED_STATE_CHANGED);
            }
        } else if self.collapsed_nodes.remove(&node_id) {
            changes.insert(MetaGraphChanges::COLLAPSED_STATE_CHANGED);
        }
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
        pending_node_operations: PendingNodeOperations,
        layout_requested: bool,
        auto_attach: bool,
        auto_layout: bool,
        changes: &mut MetaGraphChanges,
    ) {
        if let Some(selected_node_id) = self.selected_node_id
            && self
                .collapse_index
                .node_is_non_root_member_of_collapsed_subtree(selected_node_id)
        {
            self.selected_node_id = None;
        }
        if let Some(DraggingNode { node_id, .. }) = self.dragging_node
            && self
                .collapse_index
                .node_is_non_root_member_of_collapsed_subtree(node_id)
        {
            self.dragging_node = None;
        }
        if let Some(pending_edge) = self.pending_edge
            && self
                .collapse_index
                .node_is_non_root_member_of_collapsed_subtree(pending_edge.from_node)
        {
            self.pending_edge = None;
        }

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
                    .handle_drag(ui, &mut self.is_panning, &canvas_response);

                if self.pan_zoom_state.handle_scroll(ui, canvas_rect) {
                    for node in self.nodes.values_mut() {
                        node.data.prepare_text(ui, self.pan_zoom_state.zoom);
                    }
                }

                if canvas_response.clicked() {
                    if self.pending_edge.is_some() {
                        self.pending_edge = None;
                    } else if self.selected_node_id.is_some() {
                        self.selected_node_id = None;
                    }
                    if self.pending_edge.is_some() {
                        self.pending_edge = None;
                    }
                }

                // Handle name update
                if let Some(PendingNodeNameUpdate { node_id }) = pending_node_operations.name_update
                {
                    self.node_mut(node_id).data.reprepare_name_text(ui);
                    if self
                        .collapse_index
                        .node_is_visible_collapsed_subtree_root(node_id)
                    {
                        self.collapse_index
                            .update_subtree_node_size(&self.nodes, node_id);
                    }
                }

                // Handle change in kind

                if let Some(PendingNodeKindChange { node_id, kind }) =
                    pending_node_operations.kind_change
                    && self.change_node_kind(scratch, node_id, kind, changes)
                {
                    let zoom = self.pan_zoom_state.zoom;
                    self.node_mut(node_id).data.prepare_text(ui, zoom);
                }

                // Handle change in parent port count

                if let Some(PendingNodeParentPortCountChange {
                    node_id,
                    parent_port_count,
                }) = pending_node_operations.parent_port_count_change
                {
                    self.change_parent_port_count(scratch, node_id, parent_port_count, changes);
                }

                // Handle change in collapsed state

                if let Some(PendingNodeCollapsedStateChange { node_id, collapsed }) =
                    pending_node_operations.collapsed_state_change
                {
                    self.set_node_collapsed(node_id, collapsed, changes);
                }

                // `world_node_rects` and `screen_node_rects` will NOT be
                // one-to-one with `self.nodes`, since only nodes not hidden in
                // collapsed subtrees will get a rectangle
                self.compute_world_and_screen_node_rects(
                    canvas_origin,
                    &mut scratch.world_node_rects,
                    &mut scratch.screen_node_rects,
                );

                // Handle pending new node

                let mut hidden_added_node_id = None;

                if let Some(PendingNodeAddition { node_id, mut data }) =
                    pending_node_operations.addition
                {
                    data.prepare_text(ui, self.pan_zoom_state.zoom);
                    let node_size = data.compute_standard_size();

                    let position = self
                        .default_new_node_position(&scratch.world_node_rects, node_size)
                        .unwrap_or_else(|| self.output_node_position(&canvas_rect, node_size));

                    let mut node = MetaNode::new(position, data);
                    let mut world_node_rect = Rect::from_center_size(position, node_size);

                    let auto_attach_info = self.determine_auto_attach_info(node_id, &node);

                    if !auto_layout {
                        if let Some(AutoAttachInfo {
                            attach_as_child,
                            parent_node_id,
                            child_node_id,
                            ..
                        }) = &auto_attach_info
                        {
                            let new_position = if *attach_as_child {
                                self.position_relative_to_node(
                                    &scratch.world_node_rects,
                                    node_size,
                                    *parent_node_id,
                                    Direction::TopDown,
                                    Align::Center,
                                    Align::Center,
                                    NEW_NODE_GAP,
                                )
                            } else {
                                self.position_relative_to_node(
                                    &scratch.world_node_rects,
                                    node_size,
                                    *child_node_id,
                                    Direction::BottomUp,
                                    Align::Center,
                                    Align::Center,
                                    NEW_NODE_GAP,
                                )
                            }
                            .unwrap();

                            let delta = new_position - node.position;
                            node.position += delta;
                            world_node_rect = world_node_rect.translate(delta);
                        }

                        let resolve_delta = compute_delta_to_resolve_overlaps(
                            || {
                                scratch
                                    .world_node_rects
                                    .iter()
                                    .map(|(id, rect)| (*id, *rect))
                            },
                            node_id,
                            world_node_rect,
                            MIN_NODE_SEPARATION,
                        );

                        node.position += resolve_delta;
                        world_node_rect = world_node_rect.translate(resolve_delta);
                    }

                    let is_output = node.data.kind.is_output();
                    let is_leaf = node.data.kind.n_child_slots() == 0;

                    self.nodes.insert(node_id, node);
                    scratch.world_node_rects.insert(node_id, world_node_rect);

                    changes.insert(MetaGraphChanges::NODE_ADDED);

                    if auto_attach
                        && let Some(AutoAttachInfo {
                            parent_node_id,
                            child_slot_on_parent,
                            child_node_id,
                            parent_slot_on_child,
                            ..
                        }) = auto_attach_info
                    {
                        self.try_attach(
                            &mut scratch.search,
                            parent_node_id,
                            child_slot_on_parent,
                            child_node_id,
                            parent_slot_on_child,
                            changes,
                        );
                    }

                    if !is_output && (!is_leaf || self.selected_node_id.is_none()) {
                        self.selected_node_id = Some(node_id);
                    }

                    if auto_layout {
                        hidden_added_node_id = Some(node_id);
                    }
                }

                if self.is_panning {
                    self.dragging_node = None;
                }

                let mut drag_delta = None;

                for (&node_id, node) in self.nodes.iter_mut() {
                    if hidden_added_node_id == Some(node_id) {
                        continue;
                    }
                    let Some(node_rect) = scratch.screen_node_rects.get(&node_id).copied() else {
                        continue;
                    };

                    if !self.is_panning {
                        let node_response = ui.interact(
                            node_rect,
                            Id::new(("meta_node", node_id)),
                            Sense::click_and_drag(),
                        );

                        // Handle node selection

                        if node_response.clicked() && self.pending_edge.is_none() {
                            self.selected_node_id = Some(node_id);
                        }

                        // Obtain dragging delta

                        for by_button in [PointerButton::Primary, PointerButton::Secondary] {
                            if node_response.drag_started_by(by_button) {
                                self.dragging_node = Some(DraggingNode { node_id, by_button });
                            }

                            if node_response.drag_stopped_by(by_button)
                                && self.dragging_node == Some(DraggingNode { node_id, by_button })
                            {
                                self.dragging_node = None;
                            }

                            if node_response.dragged_by(by_button)
                                && self.dragging_node == Some(DraggingNode { node_id, by_button })
                            {
                                drag_delta = Some(
                                    self.pan_zoom_state
                                        .screen_vec_to_world_space(node_response.drag_delta()),
                                );
                            }
                        }
                    }

                    let is_selected = self.selected_node_id == Some(node_id);

                    let is_collapsed = self
                        .collapse_index
                        .node_is_visible_collapsed_subtree_root(node_id);

                    node.data.paint(
                        &painter,
                        node_rect,
                        self.pan_zoom_state.zoom,
                        is_selected,
                        is_collapsed,
                    );
                }

                // Draw edges

                // Start with edges fully outside any collapsed subtree
                for (&child_node_id, child_node) in &self.nodes {
                    if hidden_added_node_id == Some(child_node_id) {
                        continue;
                    }
                    if self
                        .collapse_index
                        .node_is_in_collapsed_subtree(child_node_id)
                    {
                        // Child is part of a collapsed subtree, so skip it
                        continue;
                    }
                    let Some(child_rect) = scratch.screen_node_rects.get(&child_node_id) else {
                        continue;
                    };

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
                        if hidden_added_node_id == Some(parent_node_id) {
                            continue;
                        }
                        let Some(parent_node) = self.nodes.get(&parent_node_id) else {
                            continue;
                        };
                        if self
                            .collapse_index
                            .node_is_in_collapsed_subtree(parent_node_id)
                        {
                            // Parent is part of a collapsed subtree, so skip it
                            continue;
                        }
                        let Some(parent_rect) = scratch.screen_node_rects.get(&parent_node_id)
                        else {
                            continue;
                        };

                        draw_edge(
                            &painter,
                            parent_rect,
                            child_slot_on_parent,
                            parent_node.links_to_children.len(),
                            child_rect,
                            parent_slot_on_child,
                            child_node.links_to_parents.len(),
                            parent_node.input_data_types[child_slot_on_parent],
                            child_node.output_data_type,
                            self.pan_zoom_state.zoom,
                        );
                    }
                }

                // Now draw all edges starting or ending in a collapsed subtree
                for &root_node_id in self.collapse_index.visible_collapsed_subtree_roots() {
                    let subtree = self.collapse_index.subtree(root_node_id);
                    let subtree_rect = &scratch.screen_node_rects[&root_node_id];

                    for (parent_slot_on_subtree, subtree_parent_port) in
                        subtree.exposed_parent_ports.iter().enumerate()
                    {
                        let Some(MetaNodeLink {
                            to_node: parent_node_id,
                            to_slot: child_slot_on_parent,
                        }) = subtree_parent_port.link
                        else {
                            continue;
                        };
                        if hidden_added_node_id == Some(parent_node_id) {
                            continue;
                        }
                        // The parent is guaranteed to not be part of any
                        // collapsed subtree, because if it was, this subtree
                        // would not be visible
                        let Some(parent_node) = self.nodes.get(&parent_node_id) else {
                            continue;
                        };
                        let parent_rect = &scratch.screen_node_rects[&parent_node_id];

                        draw_edge(
                            &painter,
                            parent_rect,
                            child_slot_on_parent,
                            parent_node.links_to_children.len(),
                            subtree_rect,
                            parent_slot_on_subtree,
                            subtree.exposed_parent_ports.len(),
                            parent_node.input_data_types[child_slot_on_parent],
                            subtree_parent_port.output_data_type,
                            self.pan_zoom_state.zoom,
                        );
                    }

                    for (child_slot_on_subtree, subtree_child_port) in
                        subtree.exposed_child_ports.iter().enumerate()
                    {
                        let Some(MetaNodeLink {
                            to_node: child_node_id,
                            to_slot: parent_slot_on_child,
                        }) = subtree_child_port.link
                        else {
                            continue;
                        };
                        if hidden_added_node_id == Some(child_node_id) {
                            continue;
                        }
                        // The child is guaranteed to not be part of any
                        // collapsed subtree, because if it was, it would also
                        // be part of this subtree, in which case the link would
                        // have been ignored
                        let Some(child_node) = self.nodes.get(&child_node_id) else {
                            continue;
                        };
                        let child_rect = &scratch.screen_node_rects[&child_node_id];

                        draw_edge(
                            &painter,
                            subtree_rect,
                            child_slot_on_subtree,
                            subtree.exposed_child_ports.len(),
                            child_rect,
                            parent_slot_on_child,
                            child_node.links_to_parents.len(),
                            subtree_child_port.input_data_type,
                            child_node.output_data_type,
                            self.pan_zoom_state.zoom,
                        );
                    }
                }

                // Draw ports

                let mut pending_edge_port_color = Color32::BLACK;

                for (&node_id, node_rect) in &scratch.screen_node_rects {
                    if hidden_added_node_id == Some(node_id) {
                        continue;
                    }
                    if self
                        .collapse_index
                        .node_is_visible_collapsed_subtree_root(node_id)
                    {
                        let subtree = self.collapse_index.subtree(node_id);
                        let exposed_parent_ports = subtree.exposed_parent_ports.clone();
                        let exposed_child_ports = subtree.exposed_child_ports.clone();
                        let parent_port_count = exposed_parent_ports.len();
                        let child_port_count = exposed_child_ports.len();

                        for (slot, parent_port) in exposed_parent_ports.into_iter().enumerate() {
                            let node_kind = self.node(parent_port.on_node).data.kind;

                            let port = MetaPort::Parent {
                                kind: parent_port.kind,
                                slot: parent_port.slot_on_node,
                            };
                            let position =
                                MetaPort::parent_center(node_rect, slot, parent_port_count);

                            self.handle_port(
                                &mut scratch.search,
                                ui,
                                &painter,
                                &mut pending_edge_port_color,
                                parent_port.on_node,
                                Some(node_kind),
                                port,
                                parent_port.output_data_type,
                                position,
                                changes,
                            );
                        }

                        for (slot, child_port) in exposed_child_ports.into_iter().enumerate() {
                            let node_kind = self.node(child_port.on_node).data.kind;

                            let port = MetaPort::Child {
                                kind: child_port.kind,
                                slot: child_port.slot_on_node,
                            };
                            let position =
                                MetaPort::child_center(node_rect, slot, child_port_count);

                            self.handle_port(
                                &mut scratch.search,
                                ui,
                                &painter,
                                &mut pending_edge_port_color,
                                child_port.on_node,
                                Some(node_kind),
                                port,
                                child_port.input_data_type,
                                position,
                                changes,
                            );
                        }
                    } else {
                        let node = self.node(node_id);

                        let node_kind = node.data.kind;
                        let output_data_type = node.output_data_type;
                        let input_data_types = node.input_data_types.clone();
                        let parent_port_count = node.links_to_parents.len();
                        let child_port_count = node.links_to_children.len();

                        for slot in 0..parent_port_count {
                            let port = MetaPort::Parent {
                                kind: node_kind.parent_port_kind(),
                                slot,
                            };
                            let position =
                                MetaPort::parent_center(node_rect, slot, parent_port_count);

                            self.handle_port(
                                &mut scratch.search,
                                ui,
                                &painter,
                                &mut pending_edge_port_color,
                                node_id,
                                None,
                                port,
                                output_data_type,
                                position,
                                changes,
                            );
                        }

                        for (slot, (kind, data_type)) in node_kind
                            .child_port_kinds()
                            .zip(input_data_types)
                            .enumerate()
                        {
                            let port = MetaPort::Child { kind, slot };
                            let position =
                                MetaPort::child_center(node_rect, slot, child_port_count);

                            self.handle_port(
                                &mut scratch.search,
                                ui,
                                &painter,
                                &mut pending_edge_port_color,
                                node_id,
                                None,
                                port,
                                data_type,
                                position,
                                changes,
                            );
                        }
                    }
                }

                // Draw pending edge

                if let Some(pending_edge) = &self.pending_edge
                    && let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos())
                    && let Some(node) = self.nodes.get(&pending_edge.from_node)
                    && let Some(node_rect) = scratch.screen_node_rects.get(&pending_edge.from_node)
                {
                    let from = if self
                        .collapse_index
                        .node_is_visible_collapsed_subtree_root(pending_edge.from_node)
                    {
                        self.collapse_index
                            .subtree(pending_edge.from_node)
                            .port_position(node_rect, pending_edge.from_port)
                    } else {
                        node.port_position(node_rect, pending_edge.from_port)
                    };

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
                };

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

                // Handle node deletion

                if let Some(PendingNodeRemoval { node_id }) = pending_node_operations.removal {
                    self.remove_node(scratch, node_id, changes);
                }
                if let Some(node_id) = self.selected_node_id
                    && ui.input(|i| i.key_pressed(Key::Delete))
                {
                    self.remove_node(scratch, node_id, changes);
                }

                // Update data types

                if changes.intersects(
                    MetaGraphChanges::NODE_ATTACHED
                        | MetaGraphChanges::NODE_DETACHED
                        | MetaGraphChanges::KIND_CHANGED,
                ) {
                    self.update_edge_data_types(scratch);
                }

                // Update collapsed view

                if changes.intersects(
                    MetaGraphChanges::NODE_REMOVED
                        | MetaGraphChanges::NODE_ATTACHED
                        | MetaGraphChanges::NODE_DETACHED
                        | MetaGraphChanges::KIND_CHANGED
                        | MetaGraphChanges::PARENT_PORT_COUNT_CHANGED
                        | MetaGraphChanges::COLLAPSED_STATE_CHANGED,
                ) {
                    self.rebuild_collapse_index(scratch);

                    self.compute_world_node_rects(&mut scratch.world_node_rects);
                }

                // Handle node dragging

                if let (Some(DraggingNode { node_id, by_button }), Some(delta)) =
                    (self.dragging_node, drag_delta)
                {
                    let drag_subtree = self
                        .collapse_index
                        .node_is_visible_collapsed_subtree_root(node_id)
                        || by_button == PointerButton::Secondary;

                    if drag_subtree {
                        obtain_subtree(
                            &mut scratch.search,
                            &self.nodes,
                            self.node_id_counter,
                            &mut scratch.subtree_node_ids,
                            node_id,
                        );
                        for &node_id in &scratch.subtree_node_ids {
                            translate_node(
                                &mut self.nodes,
                                &mut scratch.world_node_rects,
                                node_id,
                                delta,
                            );
                        }
                    } else {
                        translate_node(
                            &mut self.nodes,
                            &mut scratch.world_node_rects,
                            node_id,
                            delta,
                        );
                    }
                }

                // Resolve overlaps when uncollapsing subtree without auto
                // layout

                if let Some(PendingNodeCollapsedStateChange { node_id, collapsed }) =
                    pending_node_operations.collapsed_state_change
                    && !auto_layout
                    && !collapsed
                {
                    obtain_subtree(
                        &mut scratch.search,
                        &self.nodes,
                        self.node_id_counter,
                        &mut scratch.subtree_node_ids,
                        node_id,
                    );
                    for &node_id in &scratch.subtree_node_ids {
                        resolve_overlap_for_node(
                            &mut self.nodes,
                            &mut scratch.world_node_rects,
                            node_id,
                        );
                    }
                }

                // Perform layout

                if layout_requested
                    || (auto_layout
                        && changes.intersects(
                            MetaGraphChanges::NODE_ADDED
                                | MetaGraphChanges::NODE_REMOVED
                                | MetaGraphChanges::NODE_ATTACHED
                                | MetaGraphChanges::PARAMS_CHANGED
                                | MetaGraphChanges::NAME_CHANGED
                                | MetaGraphChanges::KIND_CHANGED
                                | MetaGraphChanges::COLLAPSED_STATE_CHANGED,
                        ))
                {
                    self.perform_layout(scratch);
                }
            });
    }

    fn compute_world_node_rects(&self, world_node_rects: &mut BTreeMap<MetaNodeID, Rect>) {
        world_node_rects.clear();
        for (&node_id, node) in &self.nodes {
            if let Some(world_rect) = self.compute_world_node_rect(node_id, node) {
                world_node_rects.insert(node_id, world_rect);
            }
        }
    }

    fn compute_world_and_screen_node_rects(
        &self,
        canvas_origin: Pos2,
        world_node_rects: &mut BTreeMap<MetaNodeID, Rect>,
        screen_node_rects: &mut BTreeMap<MetaNodeID, Rect>,
    ) {
        world_node_rects.clear();
        screen_node_rects.clear();
        for (&node_id, node) in &self.nodes {
            if let Some(world_rect) = self.compute_world_node_rect(node_id, node) {
                let screen_rect = self
                    .pan_zoom_state
                    .world_rect_to_screen_space(canvas_origin, world_rect);

                world_node_rects.insert(node_id, world_rect);
                screen_node_rects.insert(node_id, screen_rect);
            }
        }
    }

    fn compute_world_node_rect(&self, node_id: MetaNodeID, node: &MetaNode) -> Option<Rect> {
        let node_size = if self
            .collapse_index
            .node_is_visible_collapsed_subtree_root(node_id)
        {
            self.collapse_index.subtree(node_id).size
        } else if !self.collapse_index.node_is_in_collapsed_subtree(node_id) {
            node.data.compute_standard_size()
        } else {
            return None;
        };

        Some(Rect::from_center_size(node.position, node_size))
    }

    fn position_relative_to_node(
        &self,
        world_node_rects: &BTreeMap<MetaNodeID, Rect>,
        node_size: Vec2,
        reference_node_id: MetaNodeID,
        direction: Direction,
        x_alignment: Align,
        y_alignment: Align,
        gap: f32,
    ) -> Option<Pos2> {
        let reference_node = self.nodes.get(&reference_node_id)?;
        let reference_node_rect = world_node_rects.get(&reference_node_id)?;

        let ref_pos = reference_node.position;
        let ref_size = reference_node_rect.size();

        let base_position = match direction {
            Direction::LeftToRight => {
                let x_offset = 0.5 * (ref_size.x + node_size.x) + gap;
                ref_pos + vec2(x_offset, 0.0)
            }
            Direction::RightToLeft => {
                let x_offset = 0.5 * (ref_size.x + node_size.x) + gap;
                ref_pos - vec2(x_offset, 0.0)
            }
            Direction::TopDown => {
                let y_offset = 0.5 * (ref_size.y + node_size.y) + gap;
                ref_pos + vec2(0.0, y_offset)
            }
            Direction::BottomUp => {
                let y_offset = 0.5 * (ref_size.y + node_size.y) + gap;
                ref_pos - vec2(0.0, y_offset)
            }
        };

        let alignment_offset = match direction {
            Direction::LeftToRight | Direction::RightToLeft => {
                let y_offset = match y_alignment {
                    Align::Min => -0.5 * (ref_size.y - node_size.y),
                    Align::Center => 0.0,
                    Align::Max => 0.5 * (ref_size.y - node_size.y),
                };
                vec2(0.0, y_offset)
            }
            Direction::TopDown | Direction::BottomUp => {
                let x_offset = match x_alignment {
                    Align::Min => -0.5 * (ref_size.x - node_size.x),
                    Align::Center => 0.0,
                    Align::Max => 0.5 * (ref_size.x - node_size.x),
                };
                vec2(x_offset, 0.0)
            }
        };

        Some(base_position + alignment_offset)
    }

    fn default_new_node_position(
        &self,
        world_node_rects: &BTreeMap<MetaNodeID, Rect>,
        node_size: Vec2,
    ) -> Option<Pos2> {
        self.nodes.keys().next().and_then(|&output_node_id| {
            self.position_relative_to_node(
                world_node_rects,
                node_size,
                output_node_id,
                Direction::LeftToRight,
                Align::Center,
                Align::Min,
                NEW_NODE_GAP,
            )
        })
    }

    fn output_node_position(&self, canvas_rect: &Rect, node_size: Vec2) -> Pos2 {
        self.pan_zoom_state.screen_pos_to_world_space(
            canvas_rect.min,
            canvas_rect.center_top() + vec2(0.0, OUTPUT_NODE_TOP_CLEARING),
        ) + vec2(0.0, 0.5 * node_size.y)
    }

    fn determine_auto_attach_info(
        &self,
        node_id: MetaNodeID,
        node: &MetaNode,
    ) -> Option<AutoAttachInfo> {
        let selected_node_id = self.selected_node_id?;
        let selected_node = self.nodes.get(&selected_node_id)?;

        if let Some(free_child_slot) =
            selected_node.first_free_child_slot_accepting_type(node.output_data_type)
        {
            return Some(AutoAttachInfo {
                attach_as_child: true,
                parent_node_id: selected_node_id,
                child_slot_on_parent: free_child_slot,
                child_node_id: node_id,
                parent_slot_on_child: 0,
            });
        } else {
            for (child_slot, &input_data_type) in node.input_data_types.iter().enumerate() {
                if let Some(free_parent_slot) =
                    selected_node.first_free_parent_slot_accepting_type(input_data_type)
                {
                    return Some(AutoAttachInfo {
                        attach_as_child: false,
                        parent_node_id: node_id,
                        child_slot_on_parent: child_slot,
                        child_node_id: selected_node_id,
                        parent_slot_on_child: free_parent_slot,
                    });
                }
            }
        }
        None
    }

    fn handle_port(
        &mut self,
        search_scratch: &mut SearchScratch,
        ui: &mut Ui,
        painter: &Painter,
        pending_edge_port_color: &mut Color32,
        on_node: MetaNodeID,
        node_kind_for_label: Option<MetaNodeKind>,
        port: MetaPort,
        data_type: EdgeDataType,
        port_position: Pos2,
        changes: &mut MetaGraphChanges,
    ) {
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
                    let parent_node_id = on_node;
                    enabled = self.can_attach(
                        search_scratch,
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
                    let child_node_id = on_node;
                    enabled = self.can_attach(
                        search_scratch,
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

        let pending_edge_is_from_this_port = self
            .pending_edge
            .as_ref()
            .is_some_and(|edge| edge.from_node == on_node && edge.from_port == port);

        let port_shape = data_type.port_shape();

        let port_color = if enabled || pending_edge_is_from_this_port {
            data_type.color().standard
        } else {
            data_type.color().darker
        };

        let get_label = || {
            if let Some(kind) = node_kind_for_label {
                let (inout, slot, tofrom) = match port {
                    MetaPort::Parent { slot, .. } => ("Output", slot, "from"),
                    MetaPort::Child { slot, .. } => ("Input", slot, "to"),
                };
                Label::new(format!(
                    "{inout} {slot} {tofrom} {} ({})",
                    kind.label(),
                    data_type.port_label(),
                ))
            } else {
                Label::new(data_type.port_label())
            }
        };

        if pending_edge_is_from_this_port {
            *pending_edge_port_color = port_color;
        }

        let response = show_port(
            ui,
            painter,
            port.id(on_node),
            port_position,
            enabled,
            self.pan_zoom_state.zoom,
            self.cursor_should_be_hidden(),
            port_shape,
            port_color,
            get_label,
        );

        if response.clicked() {
            // Detach if there is a node attached to the port
            if self.pending_edge.is_none() && self.node_has_link_at_port(on_node, port) {
                match port {
                    MetaPort::Parent { slot, .. } => {
                        self.detach_parent_of(on_node, slot, changes);
                    }
                    MetaPort::Child { slot, .. } => {
                        self.detach_child_of(on_node, slot, changes);
                    }
                }

                // Create a pending edge from the clicked port
                self.pending_edge = Some(PendingEdge {
                    from_node: on_node,
                    from_port: port,
                });

                return;
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
                        let parent_node_id = on_node;
                        if self.try_attach(
                            search_scratch,
                            parent_node_id,
                            child_slot_on_parent,
                            child_node_id,
                            parent_slot_on_child,
                            changes,
                        ) {
                            self.pending_edge = None;
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
                        let child_node_id = on_node;
                        if self.try_attach(
                            search_scratch,
                            parent_node_id,
                            child_slot_on_parent,
                            child_node_id,
                            parent_slot_on_child,
                            changes,
                        ) {
                            self.pending_edge = None;
                        }
                    }
                    _ => {}
                }
            } else {
                self.pending_edge = Some(PendingEdge {
                    from_node: on_node,
                    from_port: port,
                });
            }
        }

        // Right-click to detach existing edge
        if response.clicked_by(PointerButton::Secondary)
            && self.node_has_link_at_port(on_node, port)
        {
            match port {
                MetaPort::Parent { slot, .. } => {
                    self.detach_parent_of(on_node, slot, changes);
                }
                MetaPort::Child { slot, .. } => {
                    self.detach_child_of(on_node, slot, changes);
                }
            }
        }
    }

    fn perform_layout(&mut self, scratch: &mut MetaCanvasScratch) {
        let origin = scratch
            .world_node_rects
            .get(&0)
            .map_or_else(Pos2::default, Rect::center_top);

        let mut layoutable_graph = LayoutableMetaGraph::new(
            &mut scratch.layout_lut,
            &self.nodes,
            &self.collapse_index,
            &mut scratch.world_node_rects,
        );
        layout_vertical(
            &mut scratch.layout,
            &mut layoutable_graph,
            origin,
            AUTO_LAYOUT_HORIZONTAL_GAP,
            AUTO_LAYOUT_VERTICAL_GAP,
        );

        for (&node_id, node_rect) in &scratch.world_node_rects {
            self.node_mut(node_id).position = node_rect.center();
        }
    }

    pub fn update_edge_data_types(&mut self, scratch: &mut MetaCanvasScratch) {
        update_edge_data_types(&mut scratch.data_type, &mut self.nodes);
    }

    pub fn rebuild_collapse_index(&mut self, scratch: &mut MetaCanvasScratch) {
        self.collapse_index.rebuild(
            scratch,
            &self.nodes,
            self.node_id_counter,
            &self.collapsed_nodes,
        );
    }

    pub fn save_graph<A: Allocator>(&self, arena: A, output_path: &Path) -> Result<()> {
        let mut nodes = AVec::with_capacity_in(self.nodes.len(), arena);
        nodes.extend(self.nodes.iter().map(Into::into));

        let graph = IOMetaGraphRef {
            kind: IOMetaGraphKind::Full {
                pan: self.pan_zoom_state.pan.into(),
                zoom: self.pan_zoom_state.zoom,
            },
            nodes: nodes.as_slice(),
            collapsed_nodes: &self.collapsed_nodes,
        };

        impact_io::write_ron_file(&graph, output_path)
    }

    pub fn save_subtree<A: Allocator>(
        &self,
        arena: A,
        scratch: &mut MetaCanvasScratch,
        root_node_id: MetaNodeID,
        output_path: &Path,
    ) -> Result<()> {
        obtain_subtree(
            &mut scratch.search,
            &self.nodes,
            self.node_id_counter,
            &mut scratch.subtree_node_ids,
            root_node_id,
        );

        if scratch.subtree_node_ids.is_empty() {
            bail!("Missing root node {root_node_id} when saving subtree");
        }

        let mut nodes = AVec::with_capacity_in(scratch.subtree_node_ids.len(), arena);

        nodes.extend(
            scratch
                .subtree_node_ids
                .iter()
                .map(|node_id| (node_id, &self.nodes[node_id]).into()),
        );

        let graph = IOMetaGraphRef {
            kind: IOMetaGraphKind::Subtree { root_node_id },
            nodes: nodes.as_slice(),
            collapsed_nodes: &HashSet::default(),
        };

        impact_io::write_ron_file(&graph, output_path)
    }

    pub fn load_graph(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        ui: &Ui,
        path: &Path,
    ) -> Result<()> {
        let graph: IOMetaGraph =
            impact_io::parse_ron_file(path).context("Failed to parse graph file")?;

        let IOMetaGraphKind::Full { pan, zoom } = graph.kind else {
            bail!(
                "Graph file contains a {}, not a full graph",
                graph.kind.label()
            );
        };

        let pan_zoom_state = PanZoomState::new(pan.into(), zoom.clamp(MIN_ZOOM, MAX_ZOOM));

        let mut nodes = BTreeMap::new();
        let mut node_id_counter = 0;

        for io_node in graph.nodes {
            let node_id = io_node.id;

            let mut node: MetaNode = io_node
                .try_into()
                .with_context(|| format!("Invalid node in graph file (node ID {node_id})"))?;

            node.data.prepare_text(ui, pan_zoom_state.zoom);

            nodes.insert(node_id, node);

            node_id_counter = node_id_counter.max(node_id + 1);
        }

        self.nodes = nodes;
        self.node_id_counter = node_id_counter;

        self.collapsed_nodes = graph.collapsed_nodes;

        self.pan_zoom_state = PanZoomState::new(pan.into(), zoom.clamp(MIN_ZOOM, MAX_ZOOM));

        self.selected_node_id = None;
        self.pending_edge = None;
        self.is_panning = false;
        self.dragging_node = None;

        self.update_edge_data_types(scratch);
        self.rebuild_collapse_index(scratch);

        Ok(())
    }

    pub fn load_subtree(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        ui: &Ui,
        path: &Path,
        auto_layout: bool,
    ) -> Result<()> {
        let subtree: IOMetaGraph =
            impact_io::parse_ron_file(path).context("Failed to parse subtree file")?;

        let IOMetaGraphKind::Subtree {
            root_node_id: orig_root_node_id,
        } = subtree.kind
        else {
            bail!(
                "Graph file contains a {}, not a subtree",
                subtree.kind.label()
            );
        };

        let id_offset = self.node_id_counter;

        let mut subtree_nodes = BTreeMap::new();
        let mut node_id_counter = 0;

        scratch.subtree_node_ids.clear();

        for mut io_node in subtree.nodes {
            let orig_node_id = io_node.id;

            io_node.offset_ids(id_offset);
            let node_id = io_node.id;

            let mut node: MetaNode = io_node.try_into().with_context(|| {
                format!("Invalid node in subtree file (node ID {orig_node_id})")
            })?;

            node.data.prepare_text(ui, self.pan_zoom_state.zoom);

            subtree_nodes.insert(node_id, node);
            scratch.subtree_node_ids.push(node_id);

            node_id_counter = node_id_counter.max(node_id + 1);
        }

        let root_node_id = orig_root_node_id + id_offset;
        if !subtree_nodes.contains_key(&root_node_id) {
            bail!("Subtree does not contain the root node (ID {orig_root_node_id}");
        }

        self.nodes.extend(subtree_nodes);
        self.node_id_counter = self.node_id_counter.max(node_id_counter);

        self.collapsed_nodes.insert(root_node_id);

        self.update_edge_data_types(scratch);
        self.rebuild_collapse_index(scratch);

        self.compute_world_node_rects(&mut scratch.world_node_rects);

        if auto_layout {
            self.perform_layout(scratch);
        } else {
            let root_node_size = scratch.world_node_rects[&root_node_id].size();
            let root_node_position = self
                .default_new_node_position(&scratch.world_node_rects, root_node_size)
                .unwrap();

            let orig_root_node_position = self.nodes[&root_node_id].position;
            let delta = root_node_position - orig_root_node_position;

            translate_node(
                &mut self.nodes,
                &mut scratch.world_node_rects,
                root_node_id,
                delta,
            );

            let final_delta = self.nodes[&root_node_id].position - orig_root_node_position;

            for node_id in &scratch.subtree_node_ids[1..] {
                self.nodes.get_mut(node_id).unwrap().position += final_delta;
            }
        }

        Ok(())
    }
}

impl MetaCanvasScratch {
    pub fn new() -> Self {
        Self {
            world_node_rects: BTreeMap::new(),
            screen_node_rects: BTreeMap::new(),
            subtree_node_ids: Vec::new(),
            layout_lut: MetaLayoutLookupTable::new(),
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

impl CollapseIndex {
    fn new() -> Self {
        Self {
            visible_subtree_roots: HashSet::default(),
            subtrees_by_root: HashMap::default(),
            member_to_root: HashMap::default(),
        }
    }

    fn reset(&mut self) {
        self.visible_subtree_roots.clear();
        self.subtrees_by_root.clear();
        self.member_to_root.clear();
    }

    fn rebuild(
        &mut self,
        scratch: &mut MetaCanvasScratch,
        nodes: &BTreeMap<MetaNodeID, MetaNode>,
        node_id_counter: MetaNodeID,
        collapsed_nodes: &HashSet<MetaNodeID>,
    ) {
        self.member_to_root.clear();
        self.visible_subtree_roots.clone_from(collapsed_nodes);

        for &root_node_id in collapsed_nodes {
            // If the root has already been established as a member of a
            // subtree, that subtree encompasses this one, so this one will be
            // hidden.
            if self.member_to_root.contains_key(&root_node_id) {
                continue;
            }

            // It could also be that the root is a membor of a subtree we have
            // not processed yet. If that is the case, the work we do for this
            // subtree will simply get overwritten.

            obtain_subtree(
                &mut scratch.search,
                nodes,
                node_id_counter,
                &mut scratch.subtree_node_ids,
                root_node_id,
            );

            for node_id in &scratch.subtree_node_ids[1..] {
                // If any of the non-root subtree members are also collapsed
                // roots, their subtrees should not be visible since they are
                // part of this collapsed subtree
                if collapsed_nodes.contains(node_id) {
                    self.visible_subtree_roots.remove(node_id);
                }
            }

            // Sort for consistent slot order and binary search
            scratch.subtree_node_ids.sort();

            let subtree = self
                .subtrees_by_root
                .entry(root_node_id)
                .and_modify(CollapsedMetaSubtree::clear)
                .or_default();

            subtree.size = nodes[&root_node_id]
                .data
                .compute_collapsed_size()
                .max(MIN_COLLAPSED_PROXY_NODE_SIZE);

            for &node_id in &scratch.subtree_node_ids {
                // Since we skip subtrees if we have established that they are
                // encompassed by a larger subtree and overwrite otherwise,
                // `member_to_root` will always end up with the most top-level
                // root
                self.member_to_root.insert(node_id, root_node_id);

                let node = &nodes[&node_id];

                for (slot, &link) in node.links_to_parents.iter().enumerate() {
                    if let Some(link) = link {
                        if scratch
                            .subtree_node_ids
                            .binary_search(&link.to_node)
                            .is_err()
                        {
                            // The link is to a node outside this subtree, so
                            // the port should be included among the subtree's
                            // exposed ports and the link should be preserved.
                            // The link could be to a node that is part of
                            // another subtree, but in that case the current
                            // subtree is part of that subtree and thus will not
                            // be rendered at all.
                            subtree
                                .exposed_parent_ports
                                .push(CollapsedMetaSubtreeParentPort {
                                    on_node: node_id,
                                    slot_on_node: slot,
                                    kind: node.data.kind.parent_port_kind(),
                                    output_data_type: node.output_data_type,
                                    link: Some(link),
                                });
                        }
                    } else {
                        // This is an open port, so it should be included among
                        // the subtree's exposed ports
                        subtree
                            .exposed_parent_ports
                            .push(CollapsedMetaSubtreeParentPort {
                                on_node: node_id,
                                slot_on_node: slot,
                                kind: node.data.kind.parent_port_kind(),
                                output_data_type: node.output_data_type,
                                link: None,
                            });
                    }
                }

                for (slot, ((&link, &data_type), port_kind)) in node
                    .links_to_children
                    .iter()
                    .zip(&node.input_data_types)
                    .zip(node.data.kind.child_port_kinds())
                    .enumerate()
                {
                    // There can't be a link to a child outside the subtree,
                    // since all children by definition are part of the subtree

                    if link.is_none() {
                        // This is an open port, so it should be included among
                        // the subtree's exposed ports
                        subtree
                            .exposed_child_ports
                            .push(CollapsedMetaSubtreeChildPort {
                                on_node: node_id,
                                slot_on_node: slot,
                                kind: port_kind,
                                input_data_type: data_type,
                                link: None,
                            });
                    }
                }
            }
        }
    }

    fn subtree(&self, root_node_id: MetaNodeID) -> &CollapsedMetaSubtree {
        &self.subtrees_by_root[&root_node_id]
    }

    fn update_subtree_node_size(
        &mut self,
        nodes: &BTreeMap<MetaNodeID, MetaNode>,
        root_node_id: MetaNodeID,
    ) {
        self.subtrees_by_root.get_mut(&root_node_id).unwrap().size = nodes[&root_node_id]
            .data
            .compute_collapsed_size()
            .max(MIN_COLLAPSED_PROXY_NODE_SIZE);
    }

    fn visible_collapsed_subtree_roots(&self) -> &HashSet<MetaNodeID> {
        &self.visible_subtree_roots
    }

    fn node_is_visible_collapsed_subtree_root(&self, node_id: MetaNodeID) -> bool {
        self.visible_subtree_roots.contains(&node_id)
    }

    fn node_is_in_collapsed_subtree(&self, node_id: MetaNodeID) -> bool {
        self.member_to_root.contains_key(&node_id)
    }

    fn node_is_non_root_member_of_collapsed_subtree(&self, node_id: MetaNodeID) -> bool {
        self.node_is_in_collapsed_subtree(node_id)
            && !self.node_is_visible_collapsed_subtree_root(node_id)
    }

    fn root_if_in_collapsed_subtree(&self, node_id: MetaNodeID) -> Option<MetaNodeID> {
        self.member_to_root.get(&node_id).copied()
    }
}

impl<'a> LayoutableMetaGraph<'a> {
    fn new(
        lut: &'a mut MetaLayoutLookupTable,
        all_nodes: &'a BTreeMap<MetaNodeID, MetaNode>,
        collapsed_index: &'a CollapseIndex,
        visible_node_rects: &'a mut BTreeMap<MetaNodeID, Rect>,
    ) -> Self {
        lut.build(
            all_nodes,
            collapsed_index,
            visible_node_rects.keys().copied(),
        );
        Self {
            lut: &*lut,
            visible_node_rects,
        }
    }
}

impl<'a> LayoutableGraph for LayoutableMetaGraph<'a> {
    fn n_nodes(&self) -> usize {
        self.visible_node_rects.len()
    }

    fn child_indices(&self, node_idx: usize) -> impl Iterator<Item = usize> {
        self.lut.child_indices(node_idx)
    }

    fn node_rect_mut(&mut self, node_idx: usize) -> &mut Rect {
        let node_id = self.lut.visible_node_index_map.key_at_idx(node_idx);
        self.visible_node_rects.get_mut(&node_id).unwrap()
    }
}

impl MetaLayoutLookupTable {
    fn new() -> Self {
        Self {
            visible_node_index_map: KeyIndexMapper::new(),
            child_idx_offsets_and_counts: Vec::new(),
            all_child_indices: Vec::new(),
        }
    }

    fn build(
        &mut self,
        all_nodes: &BTreeMap<MetaNodeID, MetaNode>,
        collapsed_index: &CollapseIndex,
        visible_node_ids: impl IntoIterator<Item = MetaNodeID>,
    ) {
        self.visible_node_index_map.clear();
        self.child_idx_offsets_and_counts.clear();
        self.all_child_indices.clear();

        for node_id in visible_node_ids {
            self.visible_node_index_map.push_key(node_id);
        }

        let mut offset = 0;

        for node_id in self.visible_node_index_map.key_at_each_idx() {
            if collapsed_index.node_is_visible_collapsed_subtree_root(node_id) {
                let subtree = collapsed_index.subtree(node_id);

                for child_node_idx in subtree.exposed_child_ports.iter().filter_map(|port| {
                    port.link
                        .map(|link| self.visible_node_index_map.idx(link.to_node))
                }) {
                    self.all_child_indices.push(child_node_idx);
                }
            } else {
                let node = &all_nodes[&node_id];

                for child_node_idx in node.links_to_children.iter().filter_map(|link| {
                    link.map(|link| {
                        let to_node = collapsed_index
                            .root_if_in_collapsed_subtree(link.to_node)
                            .unwrap_or(link.to_node);

                        self.visible_node_index_map.idx(to_node)
                    })
                }) {
                    self.all_child_indices.push(child_node_idx);
                }
            };

            let count = self.all_child_indices.len() - offset;
            self.child_idx_offsets_and_counts.push((offset, count));
            offset += count;
        }
    }

    fn child_indices(&self, node_idx: usize) -> impl Iterator<Item = usize> {
        let (offset, count) = self.child_idx_offsets_and_counts[node_idx];
        self.all_child_indices[offset..offset + count]
            .iter()
            .copied()
    }
}

fn node_can_reach_other(
    scratch: &mut SearchScratch,
    nodes: &BTreeMap<MetaNodeID, MetaNode>,
    node_id_counter: MetaNodeID,
    node_id: MetaNodeID,
    other_node_id: MetaNodeID,
) -> bool {
    let stack = &mut scratch.stack;
    let seen = &mut scratch.seen;

    stack.clear();
    stack.push(node_id);

    seen.resize_and_unset_all(node_id_counter as usize);

    while let Some(node_id) = stack.pop() {
        if seen.set_bit(node_id as usize) {
            continue;
        }
        if node_id == other_node_id {
            return true;
        }
        if let Some(node) = nodes.get(&node_id) {
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

fn obtain_subtree(
    scratch: &mut SearchScratch,
    nodes: &BTreeMap<MetaNodeID, MetaNode>,
    node_id_counter: MetaNodeID,
    subtree_node_ids: &mut Vec<MetaNodeID>,
    node_id: MetaNodeID,
) {
    let stack = &mut scratch.stack;
    let seen = &mut scratch.seen;

    subtree_node_ids.clear();

    stack.clear();
    stack.push(node_id);

    seen.resize_and_unset_all(node_id_counter as usize);

    while let Some(node_id) = stack.pop() {
        if seen.set_bit(node_id as usize) {
            continue;
        }
        subtree_node_ids.push(node_id);
        if let Some(node) = nodes.get(&node_id) {
            for child_node_id in node
                .links_to_children
                .iter()
                .filter_map(|link| link.map(|link| link.to_node))
            {
                stack.push(child_node_id);
            }
        }
    }
}

fn translate_node(
    nodes: &mut BTreeMap<MetaNodeID, MetaNode>,
    world_node_rects: &mut BTreeMap<MetaNodeID, Rect>,
    node_id: MetaNodeID,
    delta: Vec2,
) {
    let Some(node) = nodes.get_mut(&node_id) else {
        return;
    };
    if let Some(node_rect) = world_node_rects.get(&node_id) {
        let moved_node_rect = node_rect.translate(delta);
        let resolve_delta = compute_delta_to_resolve_overlaps(
            || world_node_rects.iter().map(|(id, rect)| (*id, *rect)),
            node_id,
            moved_node_rect,
            MIN_NODE_SEPARATION,
        );
        let final_delta = delta + resolve_delta;
        node.position += final_delta;
        *world_node_rects.get_mut(&node_id).unwrap() = node_rect.translate(final_delta);
    } else {
        node.position += delta;
    }
}

fn resolve_overlap_for_node(
    nodes: &mut BTreeMap<MetaNodeID, MetaNode>,
    world_node_rects: &mut BTreeMap<MetaNodeID, Rect>,
    node_id: MetaNodeID,
) {
    let (Some(node), Some(&node_rect)) = (nodes.get_mut(&node_id), world_node_rects.get(&node_id))
    else {
        return;
    };
    let resolve_delta = compute_delta_to_resolve_overlaps(
        || world_node_rects.iter().map(|(id, rect)| (*id, *rect)),
        node_id,
        node_rect,
        MIN_NODE_SEPARATION,
    );
    node.position += resolve_delta;
    *world_node_rects.get_mut(&node_id).unwrap() = node_rect.translate(resolve_delta);
}

fn draw_edge(
    painter: &Painter,
    parent_rect: &Rect,
    child_slot_on_parent: usize,
    parent_child_port_count: usize,
    child_rect: &Rect,
    parent_slot_on_child: usize,
    child_parent_port_count: usize,
    input_data_type: EdgeDataType,
    output_data_type: EdgeDataType,
    zoom: f32,
) {
    let edge_color = if EdgeDataType::connection_allowed(input_data_type, output_data_type) {
        input_data_type.color().standard
    } else {
        MetaPaletteColor::red().standard
    };

    let child_pos =
        MetaPort::child_center(parent_rect, child_slot_on_parent, parent_child_port_count);

    let parent_pos =
        MetaPort::parent_center(child_rect, parent_slot_on_child, child_parent_port_count);

    let edge_shape = create_bezier_edge(
        child_pos,
        parent_pos,
        PathStroke::new(EDGE_WIDTH * zoom, edge_color),
    );
    painter.add(edge_shape);
}
