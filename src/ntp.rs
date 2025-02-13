use sntpc::NtpTimestampGenerator;

#[derive(Copy, Clone)]
pub struct Timestamp {
    init_time: embassy_time::Instant,
}

impl Default for Timestamp {
    fn default() -> Self {
        Timestamp {
            init_time: embassy_time::Instant::now(),
        }
    }
}

impl NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {
        self.init_time = embassy_time::Instant::now();
    }

    fn timestamp_sec(&self) -> u64 {
        0
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        0
    }
}
