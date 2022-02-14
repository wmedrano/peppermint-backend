use command::Command;

pub mod channels;
pub mod command;
pub mod track;

pub type Id = u64;

#[derive(Copy, Clone, Debug)]
pub struct RawMidi<'a> {
    pub frame: usize,
    pub data: &'a [u8],
}

pub struct IO<'a, M> {
    pub audio_out: &'a mut channels::FixedChannels<2>,
    pub midi: M,
}

pub struct PeppermintCore {
    command_queue: ringbuf::Consumer<Command>,
    tracks: Vec<track::Track>,
}

impl PeppermintCore {
    pub fn new(command_queue: ringbuf::Consumer<Command>) -> PeppermintCore {
        PeppermintCore {
            command_queue,
            tracks: Vec::with_capacity(128),
        }
    }

    pub fn process<'a, M: Clone + Iterator<Item = RawMidi<'a>>>(
        &mut self,
        io: IO<'a, M>,
        samples: usize,
    ) {
        self.handle_command_queue();
        io.audio_out.clear();
        for track in self.tracks.iter_mut() {
            let gain = track.property(track::TrackProperty::Gain);
            io.audio_out
                .mix(track.process(samples, io.midi.clone()), gain);
        }
    }

    pub fn set_buffer_size(&mut self, buffer_size: usize) {
        for track in self.tracks.iter_mut() {
            track.set_buffer_size(buffer_size);
        }
    }

    fn handle_command_queue(&mut self) {
        self.command_queue.pop_each(
            |c| {
                match c {
                    Command::CreateTrack(t) => self.tracks.push(t),
                    Command::DeleteTrack(track_id) => {
                        self.tracks.retain(|t| t.id() != track_id);
                    }
                    Command::UpdateTrack(track_id, property, value) => {
                        for track in self.tracks.iter_mut() {
                            if track.id() == track_id {
                                track.set_property(property, value);
                                break;
                            }
                        }
                    }
                    Command::PushPluginInstance {
                        id,
                        track,
                        instance,
                        params,
                    } => {
                        if let Some(track) = self.tracks.iter_mut().find(|t| t.id() == track) {
                            track.push_instance(id, instance, params);
                        }
                    }
                    Command::DeletePluginInstance { id } => {
                        for track in self.tracks.iter_mut() {
                            track.delete_instance(id);
                        }
                    }
                };
                true
            },
            None,
        );
    }
}
