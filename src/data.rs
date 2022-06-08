extern crate anyhow;
extern crate ipfs_embed;
extern crate libipld;

use std::{str::FromStr, time::Duration};

use anyhow::{Context, Result};
use async_std::stream::StreamExt;
use futures::future::join_all;
use ipfs_embed::{
	identity::ed25519::{Keypair, SecretKey},
	Cid, DefaultParams, DefaultParams as IPFSDefaultParams, Ipfs, Key, Multiaddr, NetworkConfig,
	PeerId, StorageConfig,
};
use kate_recovery::com::{Cell, Position};

use crate::types::Event;

pub async fn init_ipfs(
	seed: u64,
	port: u16,
	path: &str,
	bootstrap_nodes: &[(PeerId, Multiaddr)],
) -> anyhow::Result<Ipfs<IPFSDefaultParams>> {
	let sweep_interval = Duration::from_secs(600);
	let path_buf = std::path::PathBuf::from_str(path).unwrap();
	let storage = StorageConfig::new(Some(path_buf), None, 10, sweep_interval);
	let mut network = NetworkConfig::new(keypair(seed));
	network.mdns = None;

	let ipfs = Ipfs::<IPFSDefaultParams>::new(ipfs_embed::Config { storage, network }).await?;

	_ = ipfs.listen_on(format!("/ip4/127.0.0.1/tcp/{}", port).parse()?)?;

	if !bootstrap_nodes.is_empty() {
		ipfs.bootstrap(bootstrap_nodes).await?;
	} else {
		// If client is the first one on the network, wait for the second client ConnectionEstablished event to use it as bootstrap
		// DHT requires boostrap to complete in order to be able to insert new records
		let node = ipfs
			.swarm_events()
			.find_map(|event| {
				if let ipfs_embed::Event::ConnectionEstablished(peer_id, connected_point) = event {
					Some((peer_id, connected_point.get_remote_address().clone()))
				} else {
					None
				}
			})
			.await
			.expect("Connection established");
		ipfs.bootstrap(&[node]).await?;
	}

	Ok(ipfs)
}

pub async fn log_ipfs_events(ipfs: Ipfs<IPFSDefaultParams>) {
	let mut events = ipfs.swarm_events();
	while let Some(event) = events.next().await {
		let event = match event {
			ipfs_embed::Event::NewListener(_) => Event::NewListener,
			ipfs_embed::Event::NewListenAddr(_, addr) => Event::NewListenAddr(addr),
			ipfs_embed::Event::ExpiredListenAddr(_, addr) => Event::ExpiredListenAddr(addr),
			ipfs_embed::Event::ListenerClosed(_) => Event::ListenerClosed,
			ipfs_embed::Event::NewExternalAddr(addr) => Event::NewExternalAddr(addr),
			ipfs_embed::Event::ExpiredExternalAddr(addr) => Event::ExpiredExternalAddr(addr),
			ipfs_embed::Event::Discovered(peer_id) => Event::Discovered(peer_id),
			ipfs_embed::Event::Unreachable(peer_id) => Event::Unreachable(peer_id),
			ipfs_embed::Event::Connected(peer_id) => Event::Connected(peer_id),
			ipfs_embed::Event::Disconnected(peer_id) => Event::Disconnected(peer_id),
			ipfs_embed::Event::Subscribed(peer_id, topic) => Event::Subscribed(peer_id, topic),
			ipfs_embed::Event::Unsubscribed(peer_id, topic) => Event::Unsubscribed(peer_id, topic),
			ipfs_embed::Event::Bootstrapped => Event::Bootstrapped,
			ipfs_embed::Event::NewInfo(peer_id) => Event::NewInfo(peer_id),
			_ => Event::Other, // TODO: Is there a purpose to handle those events?
		};
		log::trace!("Received event: {}", event);
	}
}

fn keypair(i: u64) -> Keypair {
	let mut keypair = [0; 32];
	keypair[..8].copy_from_slice(&i.to_be_bytes());
	let secret = SecretKey::from_bytes(keypair).unwrap();
	Keypair::from(secret)
}

async fn fetch_cell_from_ipfs(
	ipfs: &Ipfs<DefaultParams>,
	peers: Vec<PeerId>,
	block_number: u64,
	position: &Position,
) -> Result<Cell> {
	let reference = position.reference(block_number);
	let record_key = Key::from(reference.as_bytes().to_vec());

	log::trace!("Getting DHT record for reference {}", reference);
	let cid = ipfs
		.get_record(record_key, ipfs_embed::Quorum::One)
		.await
		.map(|record| record[0].record.value.to_vec())
		.and_then(|value| Cid::try_from(value).context("Invalid CID value"))?;

	log::trace!("Fetching IPFS block for CID {}", cid);
	ipfs.fetch(&cid, peers).await.and_then(|block| {
		Ok(Cell {
			position: position.clone(),
			content: block.data().try_into()?,
		})
	})
}

pub async fn fetch_cells_from_ipfs(
	ipfs: &Ipfs<DefaultParams>,
	block_number: u64,
	positions: &Vec<Position>,
) -> Result<(Vec<Cell>, Vec<Position>)> {
	// TODO: Should we fetch peers before loop or for each cell?
	let peers = &ipfs.peers();
	log::debug!("Number of known IPFS peers: {}", peers.len());

	if peers.is_empty() {
		log::info!("No known IPFS peers");
		return Ok((vec![], positions.to_vec()));
	}

	let res = join_all(
		positions
			.iter()
			.map(|position| fetch_cell_from_ipfs(ipfs, peers.clone(), block_number, position))
			.collect::<Vec<_>>(),
	)
	.await;

	let (fetched, unfetched): (Vec<_>, Vec<_>) = res
		.into_iter()
		.zip(positions)
		.partition(|(res, _)| res.is_ok());

	let fetched = fetched
		.into_iter()
		.map(|e| {
			let cell = e.0.unwrap();
			log::debug!("Fetched cell {} from IPFS", cell.reference(block_number));
			cell
		})
		.collect::<Vec<_>>();

	let unfetched = unfetched
		.into_iter()
		.map(|(result, position)| {
			log::debug!(
				"Error fetching cell {} from IPFS: {}",
				position.reference(block_number),
				result.unwrap_err()
			);
			position.clone()
		})
		.collect::<Vec<_>>();

	log::info!("Number of cells fetched from IPFS: {}", fetched.len());
	Ok((fetched, unfetched))
}

#[cfg(test)]
mod tests {
	// TODO
}
