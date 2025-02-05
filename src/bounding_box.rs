use embedded_graphics::prelude::Dimensions;
use embedded_graphics::prelude::Primitive;
use embedded_graphics::Drawable;

pub struct BoundingBox(());

impl BoundingBox {
    pub fn new() -> Self {
        Self(())
    }
}

impl crate::render::RenderToDisplay for BoundingBox {
    fn render_to_display(
        &mut self,
        display: &mut crate::output::OutputBuffer,
        color: embedded_graphics::pixelcolor::Rgb888,
    ) {
        let border_stroke = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .stroke_color(color)
            .stroke_width(1)
            .stroke_alignment(embedded_graphics::primitives::StrokeAlignment::Inside)
            .build();

        // Draw a px wide outline around the display.
        display
            .bounding_box()
            .into_styled(border_stroke)
            .draw(display)
            .unwrap();
    }
}
