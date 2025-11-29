#[derive(Debug, serde::Deserialize)]
pub enum Event {
    SetBrightness(u8),

    ShowText {
        duration_secs: u32,
        text: String,
    },
}
