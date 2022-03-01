use crate::{
    track::{Track, TrackProperty},
    Id,
};

pub enum Command {
    CreateTrack(Track),
    DeleteTrack(Id),
    UpdateTrack(Id, TrackProperty, f32),
    PushPluginInstance {
        id: Id,
        track: Id,
        instance: Box<livi::Instance>,
    },
    DeletePluginInstance {
        id: Id,
    },
}
