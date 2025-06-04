use embassy_net::Stack;
use embassy_time::Timer;

pub async fn wait_until_network_config_up<P, const S: usize>(
    network_stack: &Stack<'_>,
    leds: &mut embassy_rp::pio_programs::ws2812::PioWs2812<'_, P, S, { crate::konst::NUM_LEDS }>,
) where
    P: embassy_rp::pio::Instance,
{
    let mut tries = 0u32;
    while !network_stack.is_config_up() {
        Timer::after_millis(100).await;
        tries += 1;
        match tries {
            0..10 => crate::text::render_text_to_leds("NET.", crate::konst::GREEN, leds).await,
            10..20 => crate::text::render_text_to_leds("NET..", crate::konst::YELLOW, leds).await,
            20.. => crate::text::render_text_to_leds("NET...", crate::konst::RED, leds).await,
        };
    }
}

pub async fn wait_until_link_up<P, const S: usize>(
    network_stack: &Stack<'_>,
    leds: &mut embassy_rp::pio_programs::ws2812::PioWs2812<'_, P, S, { crate::konst::NUM_LEDS }>,
) where
    P: embassy_rp::pio::Instance,
{
    let mut tries = 0u32;
    while !network_stack.is_link_up() {
        Timer::after_millis(500).await;
        tries += 1;
        match tries {
            0..10 => crate::text::render_text_to_leds("DHCP.", crate::konst::GREEN, leds).await,
            10..20 => crate::text::render_text_to_leds("DHCP..", crate::konst::YELLOW, leds).await,
            20.. => crate::text::render_text_to_leds("DHCP...", crate::konst::RED, leds).await,
        };
    }
}
