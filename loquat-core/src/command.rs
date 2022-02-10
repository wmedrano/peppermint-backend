use crate::{
    track::{Track, TrackProperty},
    Id,
};

pub enum Command {
    CreateTrack(Track),
    DeleteTrack(Id),
    UpdateTrack(Id, TrackProperty, f32),
    PushPluginInstance {
        track: Id,
        instance: livi::Instance,
        params: Vec<f32>,
    },
}
