use smart_leds::RGB8;

use crate::NUM_LEDS_X;
use crate::NUM_LEDS_Y;

pub type Buffer = [[RGB8; NUM_LEDS_X]; NUM_LEDS_Y];
