use crate::{
    track::{Track, TrackProperty},
    Id,
};

#[derive(Debug)]
pub enum Command {
    CreateTrack(Track),
    DeleteTrack(Id),
    UpdateTrack(Id, TrackProperty, f32),
}
