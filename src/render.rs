pub trait Renderable {
    fn get_next_cycle_time(&self) -> embassy_time::Instant;
    fn needs_cycle(&self) -> bool;
}

pub trait RenderToDisplay {
    fn render_to_display(
        &mut self,
        display: &mut crate::output::OutputBuffer,
        color: embedded_graphics::pixelcolor::Rgb888,
    );
}
