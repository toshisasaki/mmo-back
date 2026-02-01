use crate::{AsyncRuntime, ws};
use bevy_app::{Plugin, Startup};
use bevy_ecs::prelude::*;
use quinn::{Endpoint, ServerConfig};
use std::{error::Error, sync::Arc};

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(Startup, setup_networking);
    }
}

// Resource to hold the endpoint
#[derive(Resource)]
pub struct ServerEndpoint(pub Endpoint);

fn setup_networking(mut commands: Commands, runtime: Res<AsyncRuntime>) {
    let rt = &runtime.0;

    // We need to run the async setup in the Tokio runtime
    // For now, we'll block briefly or spawn a task.
    // Since this is startup, blocking is "okay" but better to spawn and insert resource later.
    // For simplicity in this step, let's just create it.

    let endpoint = rt.block_on(async {
        bind_endpoint().await.unwrap()
    });
    
    // Spawn WebSocket server
    rt.spawn(async {
        ws::start_ws_server().await;
    });

    println!("Listening on {}", endpoint.local_addr().unwrap());
    commands.insert_resource(ServerEndpoint(endpoint));
}

/// Helper to bind Quinn endpoint
async fn bind_endpoint() -> Result<Endpoint, Box<dyn Error>> {
    let (server_config, _) = configure_server()?;
    let endpoint = Endpoint::server(server_config, "127.0.0.1:5000".parse()?)?;
    Ok(endpoint)
}

/// Dummy certificate generation for self-signed QUIC
fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_der = cert.serialize_der()?;
    let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::pki_types::PrivateKeyDer::Pkcs8(priv_key.into());
    let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der.clone())];

    let server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    Ok((server_config, cert_der))
}
