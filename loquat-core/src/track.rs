use crate::channels::FixedChannels;
use crate::Id;

#[derive(Debug)]
pub enum TrackProperty {
    Gain,
}

#[derive(Debug)]
pub struct Track {
    id: Id,
    input: FixedChannels<2>,
    output: FixedChannels<2>,
    gain: f32,
}

impl Track {
    pub fn new(id: Id, buffer_size: usize) -> Track {
        Track {
            id,
            input: FixedChannels::new(buffer_size),
            output: FixedChannels::new(buffer_size),
            gain: 1.0,
        }
    }

    pub fn set_buffer_size(&mut self, buffer_size: usize) {
        self.input.set_buffer_size(buffer_size);
        self.output.set_buffer_size(buffer_size);
    }

    pub fn set_property(&mut self, property: TrackProperty, value: f32) {
        match property {
            TrackProperty::Gain => self.gain = value,
        }
    }

    pub fn property(&self, property: TrackProperty) -> f32 {
        match property {
            TrackProperty::Gain => self.gain,
        }
    }

    pub fn process(&mut self) -> &FixedChannels<2> {
        self.input.clear();
        self.output.clear();
        &self.output
    }

    pub fn id(&self) -> Id {
        self.id
    }
}
