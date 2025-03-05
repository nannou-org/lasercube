//! Device discovery.

use crate::core;
use futures::Stream;
use lasercube_core::cmds::{Command, Response};
use lasercube_core::{cmds, port, LaserInfo};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Error type for discovery operations
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Response parse error: {0}")]
    Parse(#[from] cmds::ResponseParseError),
}

/// Discover LaserCube devices by sending a discovery packet to the given address.
///
/// This function returns a stream of `LaserInfo` structs for each LaserCube
/// that responds to the discovery query. The stream will continue producing
/// values as long as responses are received.
///
/// # Example
///
/// ```no_run
/// use futures::StreamExt;
/// use tokio::time::timeout;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let bind_ip = [0, 0, 0, 0].into();
///     let target_ip = [255, 255, 255, 255].into();
///     let mut devices = lasercube::discover::devices(bind_ip, target_ip).await?;
///
///     // Set a timeout for discovery
///     let discovery = timeout(Duration::from_secs(5), async {
///         while let Some(device_info) = devices.next().await {
///             println!("Found LaserCube: {device_info:#?}");
///         }
///     });
///
///     // Wait for timeout or completion
///     match discovery.await {
///         Ok(_) => println!("Discovery complete"),
///         Err(_) => println!("Discovery timeout"),
///     }
///
///     Ok(())
/// }
/// ```
#[tracing::instrument]
pub async fn devices(
    bind_ip: IpAddr,
    target_ip: Ipv4Addr,
) -> Result<impl Stream<Item = LaserInfo>, DiscoveryError> {
    // Create a socket for CMD port communications.
    let bind_addr = SocketAddr::new(bind_ip, port::CMD);
    tracing::debug!("Binding to UDP socket {bind_addr:?}");
    let socket = UdpSocket::bind(bind_addr).await?;

    // Enable broadcast if target is a broadcast address
    if target_ip.is_broadcast() {
        tracing::debug!("Enabling broadcast for UDP socket");
        socket.set_broadcast(true)?;
    }

    // Create a channel for the stream
    let (tx, rx) = mpsc::channel(32);

    // Create the GET_FULL_INFO command
    let cmd = Command::GetFullInfo;
    let cmd_bytes = cmd.to_bytes();

    // Send the command
    let target_addr = SocketAddrV4::new(target_ip, core::port::CMD);
    tracing::debug!("Sending GET_FULL_INFO command to {target_addr:?}");
    socket.send_to(&cmd_bytes, target_addr).await?;

    // Spawn a task to receive responses
    tokio::spawn(async move {
        // Create a buffer for receiving responses
        let mut buf = vec![0u8; 1024];
        // Track discovered devices to avoid duplicates
        let mut discovered = std::collections::HashMap::new();
        // Continuously receive responses until the channel is closed
        while !tx.is_closed() {
            let (len, _src) = match socket.recv_from(&mut buf).await {
                Ok(ok) => ok,
                Err(e) => {
                    tracing::debug!("Failed to recv on UDP socket: {e}");
                    break;
                }
            };
            let info = match Response::try_from(&buf[..len]) {
                Ok(Response::FullInfo(info)) => info,
                Ok(res) => {
                    tracing::warn!("Unexpected response: {res:?}");
                    continue;
                }
                // Failed to decode, we'll
                Err(e) => {
                    tracing::warn!("Failed to decode response: {e}");
                    continue;
                }
            };
            // If this is a new device or the info has changed, send it.
            let key = info.header.ip_addr;
            if discovered.get(&key) != Some(&info) {
                tracing::debug!("Discovered new device: {info:?}");
                discovered.insert(key, info.clone());
                // If we can't send to the channel, it's been closed
                if tx.send(info).await.is_err() {
                    tracing::debug!("Channel closed");
                    break;
                }
            }
        }
        tracing::debug!("Closing stream");
    });

    // Return the stream
    Ok(ReceiverStream::new(rx))
}
