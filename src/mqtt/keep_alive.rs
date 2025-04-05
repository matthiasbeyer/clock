pub struct MqttKeepAliver {
    pub last_keep_alive: embassy_time::Instant,
    pub keep_alive: embassy_time::Duration,
}

impl MqttKeepAliver {
    pub fn new(keep_alive: embassy_time::Duration) -> Self {
        Self {
            last_keep_alive: embassy_time::Instant::now(),
            keep_alive,
        }
    }

    pub fn as_secs(&self) -> u16 {
        self.keep_alive.as_secs() as u16
    }

    pub fn update_to_now(&mut self) {
        self.last_keep_alive = embassy_time::Instant::now();
    }
}

impl crate::render::Renderable for MqttKeepAliver {
    fn get_next_cycle_time(&self) -> embassy_time::Instant {
        self.last_keep_alive
            .checked_add(self.keep_alive / 2)
            .unwrap_or_else(embassy_time::Instant::now)
    }

    fn needs_cycle(&self) -> bool {
        self.last_keep_alive.elapsed() > (self.keep_alive / 2)
    }
}
