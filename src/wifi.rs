use cyw43::Control;

pub async fn connect<P, const S: usize>(
    control: &mut Control<'_>,
    leds: &mut embassy_rp::pio_programs::ws2812::PioWs2812<'_, P, S, { crate::konst::NUM_LEDS }>,
) where
    P: embassy_rp::pio::Instance,
{
    let mut tries = 0u32;

    while let Err(error) = control
        .join(
            crate::konst::WIFI_NETWORK,
            cyw43::JoinOptions::new(crate::konst::WIFI_PASSWORD.as_bytes()),
        )
        .await
    {
        tries += 1;
        defmt::error!("WIFI failed: {:?}", defmt::Debug2Format(&error));
        match tries {
            0..10 => crate::text::render_text_to_leds("WIFI.", crate::konst::GREEN, leds).await,
            10..20 => crate::text::render_text_to_leds("WIFI..", crate::konst::YELLOW, leds).await,
            20.. => crate::text::render_text_to_leds("WIFI...", crate::konst::YELLOW, leds).await,
        };
        embassy_time::Timer::after_millis(100).await;
    }
}
