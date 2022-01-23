pub struct RawMidi<'a> {
    pub frame: usize,
    pub data: &'a [u8],
}

pub struct IO<'a, M> {
    pub out_left: &'a mut [f32],
    pub out_right: &'a mut [f32],
    pub midi: M,
}

pub struct LoquatCore {}

impl LoquatCore {
    pub fn new() -> LoquatCore {
        LoquatCore {}
    }

    pub fn process<'a, M: Iterator<Item = RawMidi<'a>>>(&mut self, io: IO<'a, M>) {
        clear_buffer(io.out_left);
        clear_buffer(io.out_right);
    }
}

impl Default for LoquatCore {
    fn default() -> Self {
        Self::new()
    }
}

fn clear_buffer(buffer: &mut [f32]) {
    for x in buffer {
        *x = 0.0;
    }
}
