use std::sync::Mutex;

use crate::manager::PeppermintManager;
use peppermint_core::command::Command;
use ringbuf::Producer;

pub struct PeppermintServiceImpl {
    inner: Mutex<PeppermintManager>,
}

impl PeppermintServiceImpl {
    pub fn new(sample_rate: f64, buffer_size: usize, commands: Producer<Command>) -> Self {
        PeppermintServiceImpl {
            inner: Mutex::new(PeppermintManager::new(sample_rate, buffer_size, commands)),
        }
    }

    fn lock_inner(&self) -> Result<std::sync::MutexGuard<PeppermintManager>, tonic::Status> {
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

    async fn delete_plugin_instance(
        &self,
        req: tonic::Request<peppermint_proto::DeletePluginInstanceRequest>,
    ) -> Result<tonic::Response<peppermint_proto::DeletePluginInstanceResponse>, tonic::Status>
    {
        self.lock_inner()?.delete_plugin_instance(req)
    }
}
