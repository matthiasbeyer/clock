use embedded_graphics::prelude::Dimensions;
use embedded_graphics::Drawable;

use crate::render::RenderToDisplay;

pub struct Text<'t> {
    text: &'t str,
}

impl<'t> Text<'t> {
    pub fn new(text: &'t str) -> Self {
        Self { text }
    }
}

impl crate::render::RenderToDisplay for Text<'_> {
    fn render_to_display(
        &mut self,
        display: &mut crate::output::OutputBuffer,
        color: embedded_graphics::pixelcolor::Rgb888,
    ) {
        let character_style = embedded_graphics::mono_font::MonoTextStyle::new(
            &embedded_graphics::mono_font::ascii::FONT_5X8,
            color,
        );

        embedded_graphics::text::Text::with_alignment(
            self.text,
            display.bounding_box().center() + embedded_graphics::prelude::Point::new(0, 3),
            character_style,
            embedded_graphics::text::Alignment::Center,
        )
        .draw(display)
        .unwrap();
    }
}

pub async fn render_text_to_leds<P, const S: usize>(
    text: &str,
    color: embedded_graphics::pixelcolor::Rgb888,
    leds: &mut embassy_rp::pio_programs::ws2812::PioWs2812<'_, P, S, { crate::NUM_LEDS }>,
) where
    P: embassy_rp::pio::Instance,
{
    defmt::debug!("Rendering text to leds");
    let mut display = crate::output::OutputBuffer::new();
    let mut text = crate::text::Text::new(text);
    text.render_to_display(&mut display, color);
    display.render_into(leds).await;
    defmt::debug!("Rendering text to leds done");
}
