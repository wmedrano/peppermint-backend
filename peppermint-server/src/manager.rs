use std::collections::{HashMap, HashSet};

use peppermint_core::command::Command;
use ringbuf::Producer;

pub struct PeppermintManager {
    lv2_world: livi::World,
    commands: Producer<Command>,
    ids: IdManager,
    tracks: HashMap<peppermint_core::Id, peppermint_proto::Track>,
    plugin_instance_to_track: HashMap<peppermint_core::Id, peppermint_core::Id>,
    sample_rate: f64,
    buffer_size: usize,
}

impl PeppermintManager {
    pub fn new(sample_rate: f64, buffer_size: usize, commands: Producer<Command>) -> Self {
        let mut lv2_world = livi::World::new();
        lv2_world.initialize_block_length(1, 8192).unwrap();
        PeppermintManager {
            lv2_world,
            commands,
            ids: IdManager::new(),
            tracks: HashMap::new(),
            plugin_instance_to_track: HashMap::new(),
            sample_rate,
            buffer_size,
        }
    }

    fn plugin_by_id(&self, id: &str) -> Option<livi::Plugin> {
        self.lv2_world
            .iter_plugins()
            .find(|p| lv2_plugin_id(p) == id)
    }

    pub fn get_plugins(
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

    pub fn get_tracks(
        &self,
    ) -> Result<tonic::Response<peppermint_proto::GetTracksResponse>, tonic::Status> {
        let mut tracks: Vec<_> = self.tracks.values().cloned().collect();
        tracks.sort_by_key(|t| t.id);
        Ok(tonic::Response::new(peppermint_proto::GetTracksResponse {
            tracks,
        }))
    }

    pub fn create_track(
        &mut self,
        req: tonic::Request<peppermint_proto::CreateTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::CreateTrackResponse>, tonic::Status> {
        let track_id = match req.get_ref().track_id {
            0 => self.ids.next_id(),
            id => self.ids.register_id(id).ok_or_else(|| {
                tonic::Status::already_exists(format!("id {} already exists", id))
            })?,
        };
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

    pub fn delete_track(
        &mut self,
        req: tonic::Request<peppermint_proto::DeleteTrackRequest>,
    ) -> Result<tonic::Response<peppermint_proto::DeleteTrackResponse>, tonic::Status> {
        let track_id = req.get_ref().track_id;
        let track = self.tracks.remove(&track_id).ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::NotFound,
                format!("track {} not found", track_id),
            )
        })?;
        self.ids.release_id(track_id);
        for plugin_instance in track.plugin_instances.iter() {
            self.ids.release_id(plugin_instance.id);
            self.plugin_instance_to_track.remove(&plugin_instance.id);
        }
        self.commands
            .push(Command::DeleteTrack(track_id))
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to send command"))?;
        Ok(tonic::Response::new(
            peppermint_proto::DeleteTrackResponse {},
        ))
    }

    pub fn update_track(
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

    pub fn instantiate_plugin(
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
        let plugin_instance_id = self.ids.next_id();
        track
            .plugin_instances
            .push(peppermint_proto::PluginInstance {
                id: plugin_instance_id,
                plugin_id: req.get_ref().plugin_id.clone(),
                params: params.clone(),
            });
        let command = Command::PushPluginInstance {
            id: plugin_instance_id,
            track: track_core_id,
            instance,
            params,
        };
        self.commands
            .push(command)
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to send command"))?;
        Ok(tonic::Response::new(
            peppermint_proto::InstantiatePluginResponse {
                id: plugin_instance_id,
            },
        ))
    }

    pub fn delete_plugin_instance(
        &mut self,
        req: tonic::Request<peppermint_proto::DeletePluginInstanceRequest>,
    ) -> Result<tonic::Response<peppermint_proto::DeletePluginInstanceResponse>, tonic::Status>
    {
        let track_id = self
            .plugin_instance_to_track
            .get(&req.get_ref().id)
            .ok_or_else(|| {
                tonic::Status::new(
                    tonic::Code::NotFound,
                    format!("plugin instance {} not found", req.get_ref().id),
                )
            })?;
        let track = self.tracks.get_mut(track_id).ok_or_else(|| {
            tonic::Status::new(
                tonic::Code::Internal,
                format!("associated track {} not found", track_id),
            )
        })?;
        let plugin_instance_index = track
            .plugin_instances
            .iter()
            .position(|plugin_instance| plugin_instance.id == req.get_ref().id)
            .ok_or_else(|| {
                tonic::Status::new(
                    tonic::Code::NotFound,
                    format!("plugin instance {} not found", req.get_ref().id),
                )
            })?;

        self.commands
            .push(Command::DeletePluginInstance {
                id: req.get_ref().id,
            })
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to send command"))?;
        track.plugin_instances.remove(plugin_instance_index);
        self.plugin_instance_to_track.remove(&req.get_ref().id);
        self.ids.release_id(req.get_ref().id);

        Ok(tonic::Response::new(
            peppermint_proto::DeletePluginInstanceResponse {},
        ))
    }
}

pub struct IdManager {
    next_id: peppermint_core::Id,
    all_ids: HashSet<peppermint_core::Id>,
}

impl IdManager {
    pub fn new() -> IdManager {
        IdManager {
            next_id: 1,
            all_ids: HashSet::new(),
        }
    }

    pub fn next_id(&mut self) -> peppermint_core::Id {
        while self.all_ids.contains(&self.next_id) {
            self.next_id += 1;
        }
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn register_id(&mut self, id: peppermint_core::Id) -> Option<peppermint_core::Id> {
        if self.all_ids.contains(&id) {
            None
        } else {
            self.all_ids.insert(id);
            Some(id)
        }
    }

    pub fn release_id(&mut self, id: peppermint_core::Id) {
        self.all_ids.remove(&id);
    }
}

impl Default for IdManager {
    fn default() -> Self {
        Self::new()
    }
}

fn lv2_plugin_id(p: &livi::Plugin) -> String {
    format!("lv2{}", p.uri())
}
