use log::error;

use crate::channels::FixedChannels;
use crate::{Id, RawMidi};

#[derive(Debug)]
pub enum TrackProperty {
    Gain,
}

struct InstanceContainer {
    id: Id,
    instance: livi::Instance,
    params: Vec<f32>,
}

pub struct Track {
    id: Id,
    input: FixedChannels<2>,
    output: FixedChannels<2>,
    atom_input: livi::event::LV2AtomSequence,
    midi_urid: lv2_raw::LV2Urid,
    gain: f32,
    instances: Vec<InstanceContainer>,
}

impl Track {
    pub fn new(id: Id, buffer_size: usize, world: &livi::World) -> Track {
        const LV2_ATOM_SEQUENCE_SIZE: usize = 1048576; // 1MiB
        Track {
            id,
            input: FixedChannels::new(buffer_size),
            output: FixedChannels::new(buffer_size),
            atom_input: livi::event::LV2AtomSequence::new(LV2_ATOM_SEQUENCE_SIZE),
            midi_urid: world.midi_urid(),
            gain: 1.0,
            instances: Vec::with_capacity(64),
        }
    }

    pub fn push_instance(&mut self, id: Id, instance: livi::Instance, params: Vec<f32>) {
        self.instances.push(InstanceContainer {
            id,
            instance,
            params,
        });
    }

    pub fn delete_instance(&mut self, id: Id) -> Option<livi::Instance> {
        let (idx, _) = self
            .instances
            .iter()
            .enumerate()
            .find(|(_, instance)| instance.id == id)?;
        Some(self.instances.remove(idx).instance)
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

    pub fn process<'a, M>(&mut self, samples: usize, midi_input: M) -> &FixedChannels<2>
    where
        M: Iterator<Item = RawMidi<'a>>,
    {
        self.input.clear();
        self.output.clear();
        self.atom_input.clear();
        for message in midi_input {
            if let Err(e) = self.atom_input.push_midi_event::<3>(
                message.frame as i64,
                self.midi_urid,
                message.data,
            ) {
                error!("{:?}", e);
            };
        }
        for instance_container in self.instances.iter_mut() {
            std::mem::swap(&mut self.input, &mut self.output);
            let ports = livi::EmptyPortConnections::new(samples)
                .with_control_inputs(instance_container.params.iter())
                .with_audio_inputs(
                    self.input.iter_channels().take(
                        instance_container
                            .instance
                            .port_counts_for_type(livi::PortType::AudioInput),
                    ),
                )
                .with_audio_outputs(
                    self.output.iter_channels_mut().take(
                        instance_container
                            .instance
                            .port_counts_for_type(livi::PortType::AudioOutput),
                    ),
                )
                .with_atom_sequence_inputs(
                    std::iter::once(&self.atom_input).take(
                        instance_container
                            .instance
                            .port_counts_for_type(livi::PortType::AtomSequenceInput),
                    ),
                );
            if let Err(e) = unsafe { instance_container.instance.run(ports) } {
                error!("Failed to run plugin: {:?}", e);
            };
        }
        &self.output
    }

    pub fn id(&self) -> Id {
        self.id
    }
}
