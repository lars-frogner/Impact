use crate::overlay::{Corner, TextOverlay};
use impact::{
    egui::{Context, Id, Vec2, vec2},
    engine::Engine,
};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TimeOverlay;

const OFFSET_FROM_CORNER: Vec2 = vec2(10.0, 3.0);

impl TimeOverlay {
    pub(super) fn run(&mut self, ctx: &Context, engine: &Engine) {
        let simulation_time = engine.simulation_time();
        let fps = engine.current_fps();

        let text = format!("{simulation_time:.1} s    {fps:.0} FPS");

        TextOverlay::new(Id::new("time_overlay"))
            .corner(Corner::TopRight)
            .offset(OFFSET_FROM_CORNER)
            .show(ctx, &text);
    }
}
