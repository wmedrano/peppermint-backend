#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50218".parse()?;
    let loquat = Loquat::new();

    tonic::transport::Server::builder()
        .add_service(loquat_proto::loquat_server::LoquatServer::new(loquat))
        .serve(addr)
        .await?;

    Ok(())
}

struct Loquat {}

impl Loquat {
    pub fn new() -> Self {
        Loquat {}
    }
}

#[tonic::async_trait]
impl loquat_proto::loquat_server::Loquat for Loquat {
    async fn get_plugins(
        &self,
        _: tonic::Request<loquat_proto::GetPluginsRequest>,
    ) -> Result<tonic::Response<loquat_proto::GetPluginsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("Not yet implemented!"))
    }
}
