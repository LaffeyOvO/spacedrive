use crate::{
	node::{
		config::{self, P2PDiscoveryState},
		get_hardware_model_name, HardwareModel,
	},
	p2p::{libraries, OperatingSystem, SPACEDRIVE_APP_ID},
};

use sd_p2p2::{Mdns, QuicTransport, P2P};
use serde_json::json;
use std::{
	collections::HashMap,
	convert::Infallible,
	sync::{atomic::AtomicBool, Arc, Mutex, PoisonError},
};

use tokio::sync::oneshot;
use tracing::{error, info};
use uuid::Uuid;

use super::{P2PEvents, PeerMetadata};

pub struct P2PManager {
	pub(crate) p2p: Arc<P2P>,
	mdns: Mutex<Option<Mdns>>,
	quic: Mutex<Option<QuicTransport>>,
	pub(crate) events: P2PEvents,

	// TODO: Remove these from here in future PR
	pub(super) spacedrop_pairing_reqs:
		Arc<tokio::sync::Mutex<HashMap<Uuid, oneshot::Sender<Option<String>>>>>,
	pub(super) spacedrop_cancelations: Arc<tokio::sync::Mutex<HashMap<Uuid, Arc<AtomicBool>>>>,
	node_config: Arc<config::Manager>,
}

impl P2PManager {
	pub async fn new(
		node_config: Arc<config::Manager>,
		libraries: Arc<crate::library::Libraries>,
	) -> Result<Arc<P2PManager>, Infallible> {
		let p2p = P2P::new(
			SPACEDRIVE_APP_ID,
			node_config.get().await.identity,
			|stream| {
				todo!();
			},
		);
		let this = Arc::new(Self {
			p2p: p2p.clone(),
			mdns: Mutex::new(None),
			quic: Mutex::new(None),
			events: P2PEvents::spawn(p2p),
			spacedrop_pairing_reqs: Default::default(),
			spacedrop_cancelations: Default::default(),
			node_config,
		});
		this.on_node_config_change().await;

		libraries::start(this.p2p.clone(), libraries);

		info!(
			"Node RemoteIdentity('{}') libp2p::PeerId('{}') is now online listening at addresses: {:?}",
			this.p2p.remote_identity(),
			"todo", // TODO: Work this out??? // TODO: Work out libp2p `PeerId`
			this.p2p.listeners().values()
		);

		Ok(this)
	}

	// TODO: Remove this and add a subscription system to `config::Manager`
	pub async fn on_node_config_change(&self) {
		let config = self.node_config.get().await;

		PeerMetadata {
			name: config.name.clone(),
			operating_system: Some(OperatingSystem::get_os()),
			device_model: Some(get_hardware_model_name().unwrap_or(HardwareModel::Other)),
			version: Some(env!("CARGO_PKG_VERSION").to_string()),
		}
		.update(&mut self.p2p.metadata_mut());

		{
			let mut quic = self.quic.lock().unwrap_or_else(PoisonError::into_inner);

			if !config.p2p_enabled && quic.is_none() {
				let quic = match QuicTransport::spawn(self.p2p.clone()) {
					Ok(q) => *quic = Some(q),
					Err(err) => {
						error!("Failed to start P2P QUIC transport: {err}");
						let _ = self.node_config.write(|c| c.p2p_enabled = true).await;
					}
				};
			}

			if config.p2p_enabled && quic.is_some() {
				if let Some(quic) = quic.take() {
					quic.shutdown();
				}
			}
		}

		{
			let mut mdns = self.mdns.lock().unwrap_or_else(PoisonError::into_inner);

			let enabled = !config.p2p_enabled
				&& (config.p2p_discovery == P2PDiscoveryState::Everyone
					|| config.p2p_discovery == P2PDiscoveryState::ContactsOnly);

			if enabled && mdns.is_none() {
				let mdns = match Mdns::spawn(self.p2p.clone()) {
					Ok(m) => *mdns = Some(m),
					Err(err) => {
						error!("Failed to start P2P mDNS: {err}");
						// let _ = self.node_config.write(|c| c.p2p_discovery = P2PDiscoveryState::Everyone).await; // TODO: Reenable this
					}
				};
			}

			if !enabled && mdns.is_some() {
				if let Some(mdns) = mdns.take() {
					mdns.shutdown();
				}
			}
		}
	}

	pub fn state(&self) -> serde_json::Value {
		json!({
			"self_identity": self.p2p.remote_identity().to_string(),
			// "self_peer_id": self.p2p.remote_identity().to_string(), // TODO
			"metadata": self.p2p.metadata().clone(),
			"listeners": self.p2p.listeners().iter().map(|(k, v)| (k, v.addr())).collect::<HashMap<_, _>>().clone(),
			"discovered": self.p2p.discovered().clone(),
		})
	}

	pub fn shutdown(&self) {
		// `self.p2p` will automatically take care of shutting down all the hooks. Eg. `self.quic`, `self.mdns`, etc.
		self.p2p.shutdown();
	}
}
