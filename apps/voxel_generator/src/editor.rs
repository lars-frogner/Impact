mod atomic;
mod layout;
mod meta;
mod util;

use allocator_api2::alloc::Allocator;
use atomic::canvas::AtomicGraphCanvas;
use impact::{
    egui::{Button, ComboBox, Context, CursorIcon, PointerButton, Pos2, Rect, Ui, Vec2},
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
    canvas::{
        MetaCanvasScratch, MetaGraphCanvas, MetaGraphChanges, PendingNodeAddition,
        PendingNodeCollapsedStateChange, PendingNodeKindChange, PendingNodeParentPortCountChange,
        PendingNodeRemoval,
    },
    node_kind::{MetaNodeKind, MetaNodeKindGroup},
};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::editor::meta::canvas::PendingNodeOperations;

const SCROLL_SENSITIVITY: f32 = 4e-3;
const MIN_ZOOM: f32 = 0.3;
const MAX_ZOOM: f32 = 3.0;

const PARENT_PORT_COUNT_OPTIONS: [(usize, &str); 8] = [
    (1, "1"),
    (2, "2"),
    (3, "3"),
    (4, "4"),
    (5, "5"),
    (6, "6"),
    (7, "7"),
    (8, "8"),
];

#[derive(Clone, Debug)]
pub struct Editor {
    meta_graph_canvas: MetaGraphCanvas,
    meta_canvas_scratch: MetaCanvasScratch,
    atomic_graph_canvas: AtomicGraphCanvas,
    graph_dirty: bool,
    rebuild_generator: bool,
    graph_status: MetaGraphStatus,
    last_graph_path: Option<GraphPath>,
    config: EditorConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub show_editor: bool,
    pub auto_generate: bool,
    pub auto_attach: bool,
    pub auto_layout: bool,
    pub show_atomic_graph: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetaGraphStatus {
    InSync,
    Dirty,
    Invalid,
}

#[derive(Clone, Debug)]
pub struct PanZoomState {
    pan: Vec2,
    zoom: f32,
}

#[derive(Clone, Debug)]
struct GraphPath {
    path: PathBuf,
    path_string: String,
    file_stem_string: String,
}

impl Editor {
    pub fn new(config: EditorConfig) -> Self {
        Self {
            meta_graph_canvas: MetaGraphCanvas::new(),
            meta_canvas_scratch: MetaCanvasScratch::new(),
            atomic_graph_canvas: AtomicGraphCanvas::new(),
            graph_dirty: false,
            rebuild_generator: false,
            graph_status: MetaGraphStatus::Invalid,
            last_graph_path: None,
            config,
        }
    }

    pub fn build_next_voxel_sdf_generator<A>(&mut self, arena: A) -> Option<SDFVoxelGenerator>
    where
        A: Allocator + Copy,
    {
        if !(self.graph_dirty || self.rebuild_generator) {
            return None;
        }

        let Some(compiled_graph) = build::build_sdf_graph(
            arena,
            &mut self.meta_canvas_scratch.build,
            &self.meta_graph_canvas.nodes,
        ) else {
            self.graph_dirty = false;
            self.rebuild_generator = false;
            self.graph_status = MetaGraphStatus::Invalid;
            return None;
        };

        self.atomic_graph_canvas.update_nodes(&compiled_graph.graph);

        if !self.rebuild_generator {
            self.graph_status = MetaGraphStatus::Dirty;
            return None;
        }

        self.graph_dirty = false;
        self.rebuild_generator = false;

        let generator = build::build_sdf_voxel_generator(arena, compiled_graph);

        self.graph_status = MetaGraphStatus::InSync;

        Some(generator)
    }

    pub fn build_next_voxel_sdf_generator_or_default<A>(&mut self, arena: A) -> SDFVoxelGenerator
    where
        A: Allocator + Copy,
    {
        self.build_next_voxel_sdf_generator(arena)
            .unwrap_or_else(build::default_sdf_voxel_generator)
    }

    fn load_graph_from_file(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Graph (*.ron)", &["ron"])
            .set_title("Load graph")
            .pick_file()
        {
            if let Err(err) = self
                .meta_graph_canvas
                .load_graph(&mut self.meta_canvas_scratch, &path)
            {
                eprintln!("Failed to load graph: {err}");
            } else {
                self.graph_dirty = true;
                self.rebuild_generator = true;
                impact_log::info!("Loaded graph from {}", path.display());
                self.last_graph_path = Some(GraphPath::new(path));
            }
        }
    }

    fn save_graph_to_file<A: Allocator>(&mut self, arena: A) {
        if let Some(path) = FileDialog::new()
            .add_filter("Graph (*.ron)", &["ron"])
            .set_title("Save graph as")
            .save_file()
        {
            if let Err(err) = self.meta_graph_canvas.save_graph(arena, &path) {
                impact_log::error!("Failed to save graph to {}: {err:#}", path.display());
            } else {
                impact_log::info!("Saved graph to {}", path.display());
                self.last_graph_path = Some(GraphPath::new(path));
            }
        }
    }

    fn save_graph_to_last_path<A: Allocator>(&mut self, arena: A) {
        if let Some(path) = &self.last_graph_path {
            if let Err(err) = self.meta_graph_canvas.save_graph(arena, &path.path) {
                impact_log::error!("Failed to save graph to {}: {err:#}", path.path_string);
            } else {
                impact_log::info!("Saved graph to {}", path.path_string);
            }
        }
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

        let mut changes = MetaGraphChanges::empty();
        let mut layout_requested = false;

        let mut pending_node_operations = PendingNodeOperations::default();

        let zoom = self.meta_graph_canvas.zoom();

        if self.meta_graph_canvas.nodes.is_empty() {
            pending_node_operations.addition = Some(PendingNodeAddition {
                node_id: self.meta_graph_canvas.next_node_id(),
                data: MetaNodeData::new(MetaNodeKind::Output),
            });
        }

        option_panel(ctx, config, "Editor panel", |ui| {
            if let Some(path) = &self.last_graph_path {
                option_group(ui, "file", |ui| {
                    ui.label(&path.file_stem_string)
                        .on_hover_cursor(CursorIcon::Help)
                        .on_hover_text(&path.path_string);
                });
            }
            option_group(ui, "file_io", |ui| {
                if ui.button("Load…").clicked() {
                    self.load_graph_from_file();
                }

                if ui.button("Save As…").clicked() {
                    self.save_graph_to_file(arena);
                }

                if ui
                    .add_enabled(self.last_graph_path.is_some(), Button::new("Save"))
                    .clicked()
                {
                    self.save_graph_to_last_path(arena);
                };
            });

            option_group(ui, "main", |ui| {
                option_checkbox(
                    ui,
                    &mut self.config.auto_generate,
                    LabelAndHoverText::label_only("Auto generate"),
                );
                if ui.button("Generate now").clicked() {
                    self.rebuild_generator = true;
                }
                ui.end_row();
                option_checkbox(
                    ui,
                    &mut self.config.auto_attach,
                    LabelAndHoverText::label_only("Auto attach"),
                );
                option_checkbox(
                    ui,
                    &mut self.config.auto_layout,
                    LabelAndHoverText::label_only("Auto layout"),
                );
                if ui.button("Layout now").clicked() {
                    layout_requested = true;
                }
                ui.end_row();
                option_checkbox(
                    ui,
                    &mut self.config.show_atomic_graph,
                    LabelAndHoverText::label_only("Show compiled graph"),
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
                            LabelAndHoverText::label_only(kind_option.label()),
                            |ui| {
                                if ui
                                    .add_enabled(
                                        pending_node_operations.addition.is_none(),
                                        Button::new("Add"),
                                    )
                                    .clicked()
                                {
                                    pending_node_operations.addition = Some(PendingNodeAddition {
                                        node_id: self.meta_graph_canvas.next_node_id(),
                                        data: MetaNodeData::new(kind_option),
                                    });
                                }
                            },
                        );
                    }
                    ui.end_row();
                }
            });

            if let Some(selected_node_id) = self.meta_graph_canvas.selected_node_id {
                let mut is_collapsed = self
                    .meta_graph_canvas
                    .node_is_collapsed_root(selected_node_id);

                let selected_node = self.meta_graph_canvas.node_mut(selected_node_id);

                option_group(ui, "modification", |ui| {
                    if !selected_node.data.kind.is_output() {
                        let was_collapsed = is_collapsed;

                        option_checkbox(
                            ui,
                            &mut is_collapsed,
                            LabelAndHoverText::label_only("Collapsed"),
                        );

                        if is_collapsed != was_collapsed {
                            pending_node_operations.collapsed_state_change =
                                Some(PendingNodeCollapsedStateChange {
                                    node_id: selected_node_id,
                                    collapsed: is_collapsed,
                                });
                        }

                        let mut kind = selected_node.data.kind;

                        labeled_option(ui, LabelAndHoverText::label_only("Kind"), |ui| {
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
                        });

                        if kind != selected_node.data.kind {
                            pending_node_operations.kind_change = Some(PendingNodeKindChange {
                                node_id: selected_node_id,
                                kind,
                            });
                        }

                        let mut parent_port_count = selected_node.links_to_parents.len();

                        labeled_option(ui, LabelAndHoverText::label_only("Parent ports"), |ui| {
                            ComboBox::from_id_salt("parent_port_count")
                                .selected_text(PARENT_PORT_COUNT_OPTIONS[parent_port_count - 1].1)
                                .show_ui(ui, |ui| {
                                    for (option, label) in PARENT_PORT_COUNT_OPTIONS {
                                        ui.selectable_value(&mut parent_port_count, option, label);
                                    }
                                })
                        });

                        if parent_port_count != selected_node.links_to_parents.len() {
                            pending_node_operations.parent_port_count_change =
                                Some(PendingNodeParentPortCountChange {
                                    node_id: selected_node_id,
                                    parent_port_count,
                                });
                        }
                    }

                    if selected_node.data.run_controls(ui, zoom) {
                        changes.insert(MetaGraphChanges::PARAMS_CHANGED);
                    }
                });

                if !selected_node.data.kind.is_output() {
                    option_group(ui, "deletion", |ui| {
                        if ui.button("Delete node").clicked() {
                            pending_node_operations.removal = Some(PendingNodeRemoval {
                                node_id: selected_node_id,
                            });
                        }
                        ui.end_row();
                    });
                }
            }

            self.meta_graph_canvas.show(
                &mut self.meta_canvas_scratch,
                ctx,
                self.graph_status,
                pending_node_operations,
                layout_requested,
                self.config.auto_attach,
                self.config.auto_layout,
                &mut changes,
            );

            if self.config.show_atomic_graph {
                self.atomic_graph_canvas.show(arena, ctx, layout_requested);
            }

            self.graph_dirty = self.graph_dirty
                || changes.intersects(
                    MetaGraphChanges::NODE_ATTACHED
                        | MetaGraphChanges::NODE_DETACHED
                        | MetaGraphChanges::KIND_CHANGED
                        | MetaGraphChanges::PARAMS_CHANGED,
                );

            if self.config.auto_generate && self.graph_dirty {
                self.rebuild_generator = true;
            }
        });
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            show_editor: true,
            show_atomic_graph: false,
            auto_generate: true,
            auto_attach: true,
            auto_layout: true,
        }
    }
}

impl PanZoomState {
    pub fn new(pan: Vec2, zoom: f32) -> Self {
        Self { pan, zoom }
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

    pub fn handle_drag(&mut self, ui: &Ui, canvas_rect: Rect, is_panning: &mut bool) {
        // Begin pan if secondary was pressed inside canvas
        if ui.input(|i| {
            i.pointer.button_pressed(PointerButton::Secondary)
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| canvas_rect.contains(p))
        }) {
            *is_panning = true;
        }

        // End pan on release (or if no longer down)
        if ui.input(|i| {
            i.pointer.button_released(PointerButton::Secondary)
                || !i.pointer.button_down(PointerButton::Secondary)
        }) {
            *is_panning = false;
        }

        if *is_panning {
            let screen_delta = ui.input(|i| i.pointer.delta());
            self.pan += screen_delta;
        }
    }

    pub fn handle_scroll(&mut self, ui: &Ui, canvas_rect: Rect) -> bool {
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
            true
        } else {
            false
        }
    }
}

impl Default for PanZoomState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl GraphPath {
    fn new(path: PathBuf) -> Self {
        let path_string = path.display().to_string();
        let file_stem_string = path.file_stem().unwrap().display().to_string();
        Self {
            path,
            path_string,
            file_stem_string,
        }
    }
}
