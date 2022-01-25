use std::fmt::Debug;

pub struct FixedChannels<const N: usize> {
    audio: Vec<f32>,
}

impl<const N: usize> FixedChannels<N> {
    pub fn new(buffer_size: usize) -> FixedChannels<N> {
        FixedChannels {
            audio: vec![0.0; buffer_size * N],
        }
    }

    pub fn buffer_size(&self) -> usize {
        self.audio.len() / N
    }

    pub fn set_buffer_size(&mut self, buffer_size: usize) {
        self.audio.reserve(buffer_size * N);
        unsafe { self.audio.set_len(buffer_size * N) };
    }

    pub fn iter_channels(&self) -> impl ExactSizeIterator + Iterator<Item = &[f32]> {
        self.audio.chunks_exact(self.buffer_size())
    }

    pub fn iter_channels_mut(&mut self) -> impl ExactSizeIterator + Iterator<Item = &mut [f32]> {
        let buffer_size = self.buffer_size();
        self.audio.chunks_mut(buffer_size)
    }

    pub fn clear(&mut self) {
        for x in self.audio.iter_mut() {
            *x = 0.0;
        }
    }

    pub fn mix(&mut self, other: &FixedChannels<N>, gain: f32) {
        debug_assert_eq!(self.buffer_size(), other.buffer_size());
        for (x, y) in self.audio.iter_mut().zip(other.audio.iter()) {
            *x += *y * gain;
        }
    }
}

impl<const N: usize> Debug for FixedChannels<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("FixedChannels")
            .field("channels", &N)
            .field("buffer_size", &self.buffer_size())
            .finish()
    }
}
