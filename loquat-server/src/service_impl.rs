use std::collections::HashMap;
use std::sync::Mutex;

use loquat_core::command::Command;
use ringbuf::Producer;

pub struct LoquatServiceImpl {
    lv2_world: livi::World,
    commands: Mutex<Producer<Command>>,
    next_track_id: std::sync::atomic::AtomicU64,
    tracks: Mutex<HashMap<loquat_core::Id, loquat_proto::Track>>,
    buffer_size: usize,
}

impl LoquatServiceImpl {
    pub fn new(buffer_size: usize, commands: Producer<Command>) -> Self {
        let mut lv2_world = livi::World::new();
        lv2_world.initialize_block_length(1, 8192).unwrap();
        LoquatServiceImpl {
            lv2_world,
            commands: Mutex::new(commands),
            next_track_id: std::sync::atomic::AtomicU64::new(1),
            tracks: Mutex::new(HashMap::new()),
            buffer_size,
        }
    }

    pub fn send_command(&self, command: Command) -> Result<(), tonic::Status> {
        self.commands
            .try_lock()
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?
            .push(command)
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to send command"))
    }
}

#[tonic::async_trait]
impl loquat_proto::loquat_server::Loquat for LoquatServiceImpl {
    async fn get_plugins(
        &self,
        _: tonic::Request<loquat_proto::GetPluginsRequest>,
    ) -> Result<tonic::Response<loquat_proto::GetPluginsResponse>, tonic::Status> {
        let plugins = self
            .lv2_world
            .iter_plugins()
            .map(|p| loquat_proto::Plugin {
                id: format!("lv2{}", p.uri()),
                name: p.name(),
                format: loquat_proto::plugin::Format::Lv2.into(),
            })
            .collect();
        Ok(tonic::Response::new(loquat_proto::GetPluginsResponse {
            plugins,
        }))
    }

    async fn get_tracks(
        &self,
        _: tonic::Request<loquat_proto::GetTracksRequest>,
    ) -> Result<tonic::Response<loquat_proto::GetTracksResponse>, tonic::Status> {
        let mut tracks: Vec<_> = self
            .tracks
            .lock()
            .map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Internal,
                    format!("Failed to acquire lock: {}", e),
                )
            })?
            .values()
            .cloned()
            .collect();
        tracks.sort_by_key(|t| t.id);
        Ok(tonic::Response::new(loquat_proto::GetTracksResponse {
            tracks,
        }))
    }

    async fn create_track(
        &self,
        req: tonic::Request<loquat_proto::CreateTrackRequest>,
    ) -> Result<tonic::Response<loquat_proto::CreateTrackResponse>, tonic::Status> {
        let mut tracks = self
            .tracks
            .lock()
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        let mut track_id = req.get_ref().track_id;
        if track_id != 0 && tracks.contains_key(&track_id) {
            return Err(tonic::Status::new(
                tonic::Code::AlreadyExists,
                format!("track {} already exists", track_id),
            ));
        }
        while track_id == 0 || tracks.contains_key(&track_id) {
            track_id = self
                .next_track_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let core_track = loquat_core::track::Track::new(track_id, self.buffer_size);
        let track_name = if req.get_ref().name == "" {
            format!("Track{}", track_id)
        } else {
            req.get_ref().name.clone()
        };
        let proto_track = loquat_proto::Track {
            id: track_id,
            name: track_name,
            gain: core_track.property(loquat_core::track::TrackProperty::Gain),
        };
        self.send_command(Command::CreateTrack(core_track))?;
        tracks.insert(track_id, proto_track.clone());
        Ok(tonic::Response::new(loquat_proto::CreateTrackResponse {
            track: Some(proto_track),
        }))
    }

    async fn delete_track(
        &self,
        req: tonic::Request<loquat_proto::DeleteTrackRequest>,
    ) -> Result<tonic::Response<loquat_proto::DeleteTrackResponse>, tonic::Status> {
        let track_id = req.get_ref().track_id;
        let mut tracks = self
            .tracks
            .lock()
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        let _ = tracks.remove(&track_id).ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::NotFound,
                format!("track {} not found", track_id),
            )
        })?;
        self.send_command(Command::DeleteTrack(track_id))?;
        Ok(tonic::Response::new(loquat_proto::DeleteTrackResponse {}))
    }

    async fn update_track(
        &self,
        req: tonic::Request<loquat_proto::UpdateTrackRequest>,
    ) -> Result<tonic::Response<loquat_proto::UpdateTrackResponse>, tonic::Status> {
        let track_id = req.get_ref().track_id;
        let mut tracks = self
            .tracks
            .lock()
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))?;
        let track = tracks.get_mut(&track_id).ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::NotFound,
                format!("track {} not found", track_id),
            )
        })?;
        if req.get_ref().name != "" {
            track.name = req.get_ref().name.clone();
        }
        for update in req.get_ref().updates.iter() {
            let value = update.value;
            let property =
                loquat_proto::track_property_update::TrackProperty::from_i32(update.property)
                    .unwrap_or(loquat_proto::track_property_update::TrackProperty::Undefined);
            match property {
                loquat_proto::track_property_update::TrackProperty::Undefined => (),
                loquat_proto::track_property_update::TrackProperty::Gain => {
                    track.gain = value;
                    self.send_command(Command::UpdateTrack(
                        track_id,
                        loquat_core::track::TrackProperty::Gain,
                        value,
                    ))?;
                }
            }
        }
        Ok(tonic::Response::new(loquat_proto::UpdateTrackResponse {}))
    }
}
