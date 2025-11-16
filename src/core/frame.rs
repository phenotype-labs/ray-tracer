/// Frame - contains timing and pixel data
#[derive(Debug, Clone)]
pub struct Frame {
    pub number: u64,
    pub time: f32,
    pub delta: f32,
    pub pixels: Vec<u8>,
}

impl Frame {
    pub fn new(number: u64, time: f32, delta: f32, pixels: Vec<u8>) -> Self {
        Self { number, time, delta, pixels }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}
