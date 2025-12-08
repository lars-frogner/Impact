use super::{AtomicNode, AtomicPort, build::update_viewer_nodes};
use crate::editor::{
    PanZoomState,
    layout::{LayoutScratch, LayoutableGraph, compute_delta_to_resolve_overlaps, layout_vertical},
    util::create_bezier_edge,
};
use impact::egui::{
    Color32, Context, CursorIcon, Id, PointerButton, Pos2, Rect, Sense, Vec2, Window,
    epaint::PathStroke, pos2, vec2,
};
use impact_alloc::{AVec, Allocator};
use impact_voxel::generation::sdf::{SDFGraph, SDFNodeID};

const CANVAS_DEFAULT_POS: Pos2 = pos2(900.0, 22.0);
const CANVAS_DEFAULT_SIZE: Vec2 = vec2(600.0, 700.0);

const MIN_NODE_SEPARATION: f32 = 8.0;

const AUTO_LAYOUT_HORIZONTAL_GAP: f32 = 16.0;
const AUTO_LAYOUT_VERTICAL_GAP: f32 = 40.0;

const EDGE_WIDTH: f32 = 2.0;
const EDGE_COLOR: Color32 = Color32::WHITE;

/// Canvas for viewing atomic graphs (read-only).
#[derive(Clone, Debug)]
pub struct AtomicGraphCanvas {
    pan_zoom_state: PanZoomState,
    nodes: Vec<AtomicNode>,
    is_panning: bool,
    dragging_node_id: Option<SDFNodeID>,
    should_perform_layout: bool,
    layout_scratch: LayoutScratch,
}

struct LayoutableAtomicGraph<'a> {
    nodes: &'a [AtomicNode],
    rects: &'a mut [Rect],
}

impl<'a> LayoutableGraph for LayoutableAtomicGraph<'a> {
    fn n_nodes(&self) -> usize {
        self.nodes.len()
    }

    fn child_indices(&self, node_idx: usize) -> impl Iterator<Item = usize> {
        self.nodes[node_idx]
            .children
            .iter()
            .map(|child_id| *child_id as usize)
    }

    fn node_rect_mut(&mut self, node_idx: usize) -> &mut Rect {
        &mut self.rects[node_idx]
    }
}

impl AtomicGraphCanvas {
    pub fn new() -> Self {
        Self {
            pan_zoom_state: PanZoomState::default(),
            nodes: Vec::new(),
            is_panning: false,
            dragging_node_id: None,
            should_perform_layout: false,
            layout_scratch: LayoutScratch::new(),
        }
    }

    pub fn update_nodes<A: Allocator>(&mut self, graph: &SDFGraph<A>) {
        update_viewer_nodes(graph, &mut self.nodes);
        self.should_perform_layout = true;
    }

    fn cursor_should_be_hidden(&self) -> bool {
        self.is_panning || self.dragging_node_id.is_some()
    }

    pub fn show(&mut self, arena: impl Allocator, ctx: &Context, layout_requested: bool) {
        Window::new("Compiled SDF graph")
            .default_pos(CANVAS_DEFAULT_POS)
            .default_size(CANVAS_DEFAULT_SIZE)
            .vscroll(false)
            .hscroll(false)
            .show(ctx, |ui| {
                let (canvas_rect, canvas_response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::drag());

                let canvas_origin = canvas_rect.min;

                let painter = ui.painter_at(canvas_rect);

                self.pan_zoom_state
                    .handle_drag(ui, &mut self.is_panning, &canvas_response);

                self.pan_zoom_state.handle_scroll(ui, canvas_rect);

                let mut world_node_rects = AVec::with_capacity_in(self.nodes.len(), arena);
                for node in &mut self.nodes {
                    node.data.prepare_text(ui, self.pan_zoom_state.zoom);

                    world_node_rects
                        .push(Rect::from_min_size(node.position, node.data.compute_size()));
                }

                if self.should_perform_layout || layout_requested {
                    let origin = self
                        .pan_zoom_state
                        .screen_pos_to_world_space(canvas_origin, canvas_rect.center_top());

                    layout_vertical(
                        &mut self.layout_scratch,
                        &mut LayoutableAtomicGraph {
                            nodes: &self.nodes,
                            rects: &mut world_node_rects,
                        },
                        origin,
                        AUTO_LAYOUT_HORIZONTAL_GAP,
                        AUTO_LAYOUT_VERTICAL_GAP,
                    );

                    for (node, node_rect) in self.nodes.iter_mut().zip(&world_node_rects) {
                        node.position = node_rect.min;
                    }

                    self.should_perform_layout = false;
                }

                for (node_idx, (node, &world_node_rect)) in
                    self.nodes.iter_mut().zip(&world_node_rects).enumerate()
                {
                    let node_id = node_idx as SDFNodeID;

                    let node_rect = self
                        .pan_zoom_state
                        .world_rect_to_screen_space(canvas_origin, world_node_rect);

                    // Handle node dragging

                    if self.is_panning {
                        self.dragging_node_id = None;
                    } else {
                        let node_response = ui.interact(
                            node_rect,
                            Id::new(("atomic_node", node_id)),
                            Sense::drag(),
                        );

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
                                || {
                                    world_node_rects
                                        .iter()
                                        .enumerate()
                                        .map(|(idx, rect)| (idx as SDFNodeID, *rect))
                                },
                                node_id,
                                moved_node_rect,
                                MIN_NODE_SEPARATION,
                            );

                            node.position += delta + resolve_delta;
                        }
                    }

                    node.data
                        .paint(&painter, node_rect, self.pan_zoom_state.zoom);
                }

                // We will only need node rects in screen space from now
                for node_rect in &mut world_node_rects {
                    *node_rect = self
                        .pan_zoom_state
                        .world_rect_to_screen_space(canvas_origin, *node_rect);
                }
                let node_rects = world_node_rects;

                // Draw edges

                for (node_idx, node) in self.nodes.iter().enumerate() {
                    let node_id = node_idx as SDFNodeID;

                    for &parent_node_id in &node.parents {
                        let parent_rect = &node_rects[parent_node_id as usize];
                        let parent_node = &self.nodes[parent_node_id as usize];
                        let node_rect = &node_rects[node_idx];

                        for slot in parent_node
                            .children
                            .iter()
                            .enumerate()
                            .filter_map(|(slot, child)| (*child == node_id).then_some(slot))
                        {
                            let child_pos = AtomicPort::Child {
                                slot,
                                of: parent_node.children.len(),
                            }
                            .center(parent_rect);

                            let parent_pos = AtomicPort::Parent.center(node_rect);

                            let edge_shape = create_bezier_edge(
                                child_pos,
                                parent_pos,
                                PathStroke::new(EDGE_WIDTH * self.pan_zoom_state.zoom, EDGE_COLOR),
                            );
                            painter.add(edge_shape);
                        }
                    }
                }

                // Draw ports

                for (node_idx, node_rect) in node_rects.iter().enumerate() {
                    for port in self.nodes[node_idx].data.kind.port_config().ports() {
                        port.paint(&painter, node_rect, self.pan_zoom_state.zoom);
                    }
                }

                // Potentially hide cursor

                if self.cursor_should_be_hidden() {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::None);
                }
            });
    }
}
