use std::collections::HashMap;
use std::sync::Mutex;

use peppermint_core::command::Command;
use ringbuf::Producer;

pub struct PeppermintServiceImpl {
    inner: Mutex<PeppermintServiceImplInner>,
}

impl PeppermintServiceImpl {
    pub fn new(sample_rate: f64, buffer_size: usize, commands: Producer<Command>) -> Self {
        PeppermintServiceImpl {
            inner: Mutex::new(PeppermintServiceImplInner::new(
                sample_rate,
                buffer_size,
                commands,
            )),
        }
    }

    fn lock_inner(
        &self,
    ) -> Result<std::sync::MutexGuard<PeppermintServiceImplInner>, tonic::Status> {
        self.inner
            .lock()
            .map_err(|e| tonic::Status::new(tonic::Code::Internal, e.to_string()))
    }
}

#[tonic::async_trait]
impl peppermint_proto::peppermint_server::Peppermint for PeppermintServiceImpl {
    async fn get_plugins(
        &self,
        _: tonic::Request<peppermint_proto::GetPluginsRequest>,
    ) -> Result<tonic::Response<peppermint_proto::GetPluginsResponse>, tonic::Status> {
        self.lock_inner()?.get_plugins()
    }

    async fn get_tracks(
        &self,
        _: tonic::Request<peppermint_proto::GetTracksRequest>,
    ) -> Result<tonic::Response<peppermint_proto::GetTracksResponse>, tonic::Status> {
        self.lock_inner()?.get_tracks()
    }

    async fn create_track(
        &self,
        req: tonic::Request<peppermint_proto::CreateTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::CreateTrackResponse>, tonic::Status> {
        self.lock_inner()?.create_track(req)
    }

    async fn delete_track(
        &self,
        req: tonic::Request<peppermint_proto::DeleteTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::DeleteTrackResponse>, tonic::Status> {
        self.lock_inner()?.delete_track(req)
    }

    async fn update_track(
        &self,
        req: tonic::Request<peppermint_proto::UpdateTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::UpdateTrackResponse>, tonic::Status> {
        self.lock_inner()?.update_track(req)
    }

    async fn instantiate_plugin(
        &self,
        req: tonic::Request<peppermint_proto::InstantiatePluginRequest>,
    ) -> Result<tonic::Response<peppermint_proto::InstantiatePluginResponse>, tonic::Status> {
        self.lock_inner()?.instantiate_plugin(req)
    }
}

pub struct PeppermintServiceImplInner {
    lv2_world: livi::World,
    commands: Producer<Command>,
    next_track_id: peppermint_core::Id,
    tracks: HashMap<peppermint_core::Id, peppermint_proto::Track>,
    sample_rate: f64,
    buffer_size: usize,
}

impl PeppermintServiceImplInner {
    fn new(sample_rate: f64, buffer_size: usize, commands: Producer<Command>) -> Self {
        let mut lv2_world = livi::World::new();
        lv2_world.initialize_block_length(1, 8192).unwrap();
        PeppermintServiceImplInner {
            lv2_world,
            commands,
            next_track_id: 1,
            tracks: HashMap::new(),
            sample_rate,
            buffer_size,
        }
    }

    fn plugin_by_id(&self, id: &str) -> Option<livi::Plugin> {
        self.lv2_world
            .iter_plugins()
            .find(|p| lv2_plugin_id(p) == id)
    }

    fn get_plugins(
        &self,
    ) -> Result<tonic::Response<peppermint_proto::GetPluginsResponse>, tonic::Status> {
        let plugins = self
            .lv2_world
            .iter_plugins()
            .map(|plugin| peppermint_proto::Plugin {
                id: lv2_plugin_id(&plugin),
                name: plugin.name(),
                format: peppermint_proto::plugin::Format::Lv2.into(),
                params: plugin
                    .ports_with_type(livi::PortType::ControlInput)
                    .enumerate()
                    .map(|(index, port)| peppermint_proto::PluginParam {
                        name: port.name.clone(),
                        default_value: port.default_value,
                        index: index as u32,
                    })
                    .collect(),
            })
            .collect();
        Ok(tonic::Response::new(peppermint_proto::GetPluginsResponse {
            plugins,
        }))
    }

    fn get_tracks(
        &self,
    ) -> Result<tonic::Response<peppermint_proto::GetTracksResponse>, tonic::Status> {
        let mut tracks: Vec<_> = self.tracks.values().cloned().collect();
        tracks.sort_by_key(|t| t.id);
        Ok(tonic::Response::new(peppermint_proto::GetTracksResponse {
            tracks,
        }))
    }

    fn create_track(
        &mut self,
        req: tonic::Request<peppermint_proto::CreateTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::CreateTrackResponse>, tonic::Status> {
        let mut track_id = req.get_ref().track_id;
        if track_id != 0 && self.tracks.contains_key(&track_id) {
            return Err(tonic::Status::new(
                tonic::Code::AlreadyExists,
                format!("track {} already exists", track_id),
            ));
        }
        while track_id == 0 || self.tracks.contains_key(&track_id) {
            track_id = self.next_track_id;
            self.next_track_id += 1;
        }
        let core_track =
            peppermint_core::track::Track::new(track_id, self.buffer_size, &self.lv2_world);
        let track_name = if req.get_ref().name.is_empty() {
            format!("Track{}", track_id)
        } else {
            req.get_ref().name.clone()
        };
        let proto_track = peppermint_proto::Track {
            id: track_id,
            name: track_name,
            gain: core_track.property(peppermint_core::track::TrackProperty::Gain),
            plugin_instances: Vec::new(),
        };
        self.commands
            .push(Command::CreateTrack(core_track))
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to send command"))?;
        self.tracks.insert(track_id, proto_track.clone());
        Ok(tonic::Response::new(
            peppermint_proto::CreateTrackResponse {
                track: Some(proto_track),
            },
        ))
    }

    fn delete_track(
        &mut self,
        req: tonic::Request<peppermint_proto::DeleteTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::DeleteTrackResponse>, tonic::Status> {
        let track_id = req.get_ref().track_id;
        let _ = self.tracks.remove(&track_id).ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::NotFound,
                format!("track {} not found", track_id),
            )
        })?;
        self.commands
            .push(Command::DeleteTrack(track_id))
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to send command"))?;
        Ok(tonic::Response::new(
            peppermint_proto::DeleteTrackResponse {},
        ))
    }

    fn update_track(
        &mut self,
        req: tonic::Request<peppermint_proto::UpdateTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::UpdateTrackResponse>, tonic::Status> {
        let track_id = req.get_ref().track_id;
        let track = self.tracks.get_mut(&track_id).ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::NotFound,
                format!("track {} not found", track_id),
            )
        })?;
        if !req.get_ref().name.is_empty() {
            track.name = req.get_ref().name.clone();
        }
        for update in req.get_ref().updates.iter() {
            let value = update.value;
            let property =
                peppermint_proto::track_property_update::TrackProperty::from_i32(update.property)
                    .unwrap_or(peppermint_proto::track_property_update::TrackProperty::Undefined);
            match property {
                peppermint_proto::track_property_update::TrackProperty::Undefined => (),
                peppermint_proto::track_property_update::TrackProperty::Gain => {
                    track.gain = value;
                    let command = Command::UpdateTrack(
                        track_id,
                        peppermint_core::track::TrackProperty::Gain,
                        value,
                    );
                    self.commands.push(command).map_err(|_| {
                        tonic::Status::new(tonic::Code::Internal, "failed to send command")
                    })?;
                }
            }
        }
        Ok(tonic::Response::new(
            peppermint_proto::UpdateTrackResponse {},
        ))
    }

    fn instantiate_plugin(
        &mut self,
        req: tonic::Request<peppermint_proto::InstantiatePluginRequest>,
    ) -> Result<tonic::Response<peppermint_proto::InstantiatePluginResponse>, tonic::Status> {
        let plugin = self.plugin_by_id(&req.get_ref().plugin_id).ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::NotFound,
                format!("plugin {} not found", req.get_ref().plugin_id),
            )
        })?;
        let instance = unsafe {
            plugin.instantiate(self.sample_rate as f64).map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Internal,
                    format!("failed to instantiate plugin: {:?}", e),
                )
            })?
        };
        let track = self
            .tracks
            .get_mut(&req.get_ref().track_id)
            .ok_or_else(|| {
                tonic::Status::new(
                    tonic::Code::NotFound,
                    format!("track {} not found", req.get_ref().track_id),
                )
            })?;
        let track_core_id = track.id;
        let params: Vec<f32> = plugin
            .ports_with_type(livi::PortType::ControlInput)
            .map(|port| port.default_value)
            .collect();
        let plugin_instance_index = track.plugin_instances.len() as u64;
        track
            .plugin_instances
            .push(peppermint_proto::PluginInstance {
                plugin_id: req.get_ref().plugin_id.clone(),
                params: params.clone(),
            });
        let command = Command::PushPluginInstance {
            track: track_core_id,
            instance,
            params,
        };
        self.commands
            .push(command)
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to send command"))?;
        Ok(tonic::Response::new(
            peppermint_proto::InstantiatePluginResponse {
                track_id: track_core_id,
                plugin_instance_index,
            },
        ))
    }
}

fn lv2_plugin_id(p: &livi::Plugin) -> String {
    format!("lv2{}", p.uri())
}
