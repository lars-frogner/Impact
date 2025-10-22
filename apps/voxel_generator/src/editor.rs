mod atomic;
mod layout;
mod meta;

use allocator_api2::alloc::Allocator;
use atomic::canvas::AtomicGraphCanvas;
use impact::{
    egui::{Button, ComboBox, Context, PointerButton, Pos2, Rect, Response, Ui, Vec2},
    engine::Engine,
};
use impact_dev_ui::{
    CustomPanels, UserInterfaceConfig as DevUserInterfaceConfig,
    option_panels::{
        LabelAndHoverText, labeled_option, option_checkbox, option_group, option_panel,
    },
};
use impact_voxel::generation::SDFVoxelGenerator;
use meta::{
    MetaNodeData, build,
    canvas::MetaGraphCanvas,
    node_kind::{MetaNodeKind, MetaNodeKindGroup},
};
use serde::{Deserialize, Serialize};

const SCROLL_SENSITIVITY: f32 = 4e-3;
const MIN_ZOOM: f32 = 0.3;
const MAX_ZOOM: f32 = 3.0;

#[derive(Clone, Debug)]
pub struct Editor {
    meta_graph_canvas: MetaGraphCanvas,
    atomic_graph_canvas: AtomicGraphCanvas,
    needs_rebuild: bool,
    graph_status: MetaGraphStatus,
    config: EditorConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub show_editor: bool,
    pub show_atomic_graph: bool,
    pub auto_attach: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetaGraphStatus {
    Complete,
    Incomplete,
}

#[derive(Clone, Debug)]
pub struct PanZoomState {
    pan: Vec2,
    zoom: f32,
}

impl Editor {
    pub fn new(config: EditorConfig) -> Self {
        Self {
            meta_graph_canvas: MetaGraphCanvas::new(),
            atomic_graph_canvas: AtomicGraphCanvas::new(),
            needs_rebuild: true,
            graph_status: MetaGraphStatus::Incomplete,
            config,
        }
    }

    pub fn build_next_voxel_sdf_generator<A>(&mut self, arena: A) -> Option<SDFVoxelGenerator>
    where
        A: Allocator + Copy,
    {
        if !self.needs_rebuild {
            return None;
        }
        self.needs_rebuild = false;

        let Some(compiled_graph) = build::build_sdf_graph(arena, &self.meta_graph_canvas.nodes)
        else {
            self.graph_status = MetaGraphStatus::Incomplete;
            return None;
        };

        self.atomic_graph_canvas.update_nodes(&compiled_graph.graph);

        let generator = build::build_sdf_voxel_generator(arena, compiled_graph);

        self.graph_status = MetaGraphStatus::Complete;

        Some(generator)
    }

    pub fn build_next_voxel_sdf_generator_or_default<A>(&mut self, arena: A) -> SDFVoxelGenerator
    where
        A: Allocator + Copy,
    {
        self.build_next_voxel_sdf_generator(arena)
            .unwrap_or_else(build::default_sdf_voxel_generator)
    }
}

impl CustomPanels for Editor {
    fn run_toolbar_buttons(&mut self, ui: &mut Ui) {
        ui.toggle_value(&mut self.config.show_editor, "Voxel editor");
    }

    fn run_panels<A>(
        &mut self,
        arena: A,
        ctx: &Context,
        config: &DevUserInterfaceConfig,
        _engine: &Engine,
    ) where
        A: Allocator + Copy,
    {
        if !self.config.show_editor {
            return;
        }

        let mut connectivity_may_have_changed = false;
        let mut params_changed = false;

        let mut pending_new_node = if self.meta_graph_canvas.nodes.is_empty() {
            Some((
                self.meta_graph_canvas.next_node_id(),
                MetaNodeData::new(MetaNodeKind::Output),
            ))
        } else {
            None
        };

        option_panel(ctx, config, "Editor panel", |ui| {
            option_group(ui, "main", |ui| {
                option_checkbox(
                    ui,
                    &mut self.config.show_atomic_graph,
                    LabelAndHoverText {
                        label: "Show compiled graph",
                        hover_text: "",
                    },
                );
            });

            option_group(ui, "creation", |ui| {
                for kind_group in MetaNodeKindGroup::all_non_root() {
                    for kind_option in MetaNodeKind::all_non_root() {
                        if kind_option.group() != kind_group {
                            continue;
                        }
                        labeled_option(
                            ui,
                            LabelAndHoverText {
                                label: kind_option.label(),
                                hover_text: "",
                            },
                            |ui| {
                                if ui
                                    .add_enabled(pending_new_node.is_none(), Button::new("Add"))
                                    .clicked()
                                {
                                    pending_new_node = Some((
                                        self.meta_graph_canvas.next_node_id(),
                                        MetaNodeData::new(kind_option),
                                    ));
                                }
                            },
                        );
                    }
                    ui.end_row();
                }
            });

            option_group(ui, "modification", |ui| {
                if let Some(selected_node_id) = self.meta_graph_canvas.selected_node_id {
                    let mut selected_node = self
                        .meta_graph_canvas
                        .nodes
                        .get_mut(&selected_node_id)
                        .unwrap();
                    let mut kind = selected_node.data.kind;

                    labeled_option(
                        ui,
                        LabelAndHoverText {
                            label: "Kind",
                            hover_text: "",
                        },
                        |ui| {
                            ui.add_enabled_ui(!selected_node.data.kind.is_root(), |ui| {
                                ComboBox::from_id_salt("selected_kind")
                                    .selected_text(selected_node.data.kind.label())
                                    .show_ui(ui, |ui| {
                                        for kind_option in MetaNodeKind::all_non_root() {
                                            ui.selectable_value(
                                                &mut kind,
                                                kind_option,
                                                kind_option.label(),
                                            );
                                        }
                                    })
                            })
                        },
                    );

                    if kind != selected_node.data.kind {
                        self.meta_graph_canvas
                            .change_node_kind(selected_node_id, kind);
                        selected_node = self.meta_graph_canvas.node_mut(selected_node_id);
                        connectivity_may_have_changed = true;
                    }

                    for param in &mut selected_node.data.params {
                        if param.show_controls(ui).changed() {
                            params_changed = true;
                        };
                    }
                }
            });

            option_group(ui, "deletion", |ui| {
                if let Some(selected_node_id) = self.meta_graph_canvas.selected_node_id {
                    let selected_node =
                        self.meta_graph_canvas.nodes.get(&selected_node_id).unwrap();

                    if ui
                        .add_enabled(
                            !selected_node.data.kind.is_root(),
                            Button::new("Delete node"),
                        )
                        .clicked()
                    {
                        self.meta_graph_canvas.remove_node(selected_node_id);
                        connectivity_may_have_changed = true;
                    }
                    ui.end_row();
                } else {
                    ui.add_enabled(false, Button::new("Delete node"));
                    ui.end_row();
                }
            });

            let canvas_result = self.meta_graph_canvas.show(
                ctx,
                self.graph_status,
                pending_new_node,
                self.config.auto_attach,
            );

            if self.config.show_atomic_graph {
                self.atomic_graph_canvas.show(arena, ctx);
            }

            connectivity_may_have_changed =
                connectivity_may_have_changed || canvas_result.connectivity_may_have_changed;

            self.needs_rebuild =
                self.needs_rebuild || connectivity_may_have_changed || params_changed;
        });
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            show_editor: true,
            show_atomic_graph: false,
            auto_attach: true,
        }
    }
}

impl PanZoomState {
    pub fn new() -> Self {
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

    pub fn handle_drag(&mut self, response: &Response) {
        if response.dragged_by(PointerButton::Secondary) {
            self.pan += response.drag_delta();
        }
    }

    pub fn handle_scroll(&mut self, ui: &Ui, canvas_rect: Rect) {
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

impl Default for PanZoomState {
    fn default() -> Self {
        Self::new()
    }
}
