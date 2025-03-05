use lasercube_core::{
    cmds::{Command, CommandType, Response, ResponseParseError},
    port,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use thiserror::Error;
use tokio::net::UdpSocket;

/// Error types that can occur when interacting with a LaserCube device
#[derive(Debug, Error)]
pub enum CommandError {
    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to parse the response.
    #[error("Response parse error: {0}")]
    Parse(#[from] ResponseParseError),
    /// Received an unexpected response.
    #[error("Unexpected response: expected command type {expected:?}, got {actual}")]
    UnexpectedResponse { expected: CommandType, actual: u8 },
}

/// A client for sending commands to a specific LaserCube device.
#[derive(Debug)]
pub struct Client {
    /// Socket for sending commands
    socket: UdpSocket,
    /// Target address for the device
    target_addr: SocketAddrV4,
}

impl Client {
    /// Create a new Client from a single target device IP (non-broadcast).
    ///
    /// Returns a new Client or an error if the socket couldn't be created.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // First discover devices
    ///     let bind_ip = [0, 0, 0, 0].into();
    ///     let target_ip = [255, 255, 255, 255].into();
    ///     let mut devices = lasercube::discover::devices(bind_ip, target_ip).await?;
    ///
    ///     // Connect to the first device found
    ///     if let Some(device_info) = devices.next().await {
    ///         let client = lasercube::Client::new(bind_ip, device_info.header.ip_addr).await?;
    ///
    ///         // Now you can send commands to the device
    ///         let buffer_free = client.get_buffer_free().await?;
    ///         println!("Buffer free: {buffer_free}");
    ///
    ///         // Enable output
    ///         client.set_output(true).await?;
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument]
    pub async fn new(bind_ip: IpAddr, target_ip: Ipv4Addr) -> Result<Self, CommandError> {
        // Create a socket for CMD port communications
        let bind_addr = SocketAddr::new(bind_ip, 0); // Use ephemeral port
        tracing::debug!("Binding to UDP socket {bind_addr:?} for commands");
        let socket = UdpSocket::bind(bind_addr).await?;
        // Set up the target address
        let target_addr = SocketAddrV4::new(target_ip.into(), port::CMD);
        // Create the client
        let client = Client {
            socket,
            target_addr,
        };
        Ok(client)
    }

    /// Send a command to the LaserCube and wait for a response.
    ///
    /// This method will await until a response is received.
    ///
    /// Returns the parsed response, or an error in the case that an
    /// I/O issue occurred or an unexpected response was received.
    #[tracing::instrument(skip(self, command))]
    pub async fn send_command(&self, command: Command) -> Result<Response, CommandError> {
        // Get command type.
        let command_type = command.command_type();
        // Create a buffer for the response.
        let mut buf = vec![0u8; 1024];
        // Send the command.
        let cmd_bytes = command.to_bytes();
        tracing::debug!("Sending command {:?} to {}", command_type, self.target_addr);
        self.socket.send_to(&cmd_bytes, self.target_addr).await?;
        let (len, _src) = self.socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        // Verify the response is for the command we sent.
        if len > 0 && data[0] == command_type as u8 {
            // Parse the response.
            match Response::try_from(data) {
                Ok(response) => Ok(response),
                Err(e) => Err(CommandError::Parse(e)),
            }
        } else if len > 0 {
            // We received a response, but it's for a different command.
            Err(CommandError::UnexpectedResponse {
                expected: command_type,
                actual: data[0],
            })
        } else {
            // Received an empty response
            Err(CommandError::Parse(ResponseParseError::EmptyResponse))
        }
    }

    /// Get the amount of free space in the device's buffer.
    ///
    /// Returns the number of free points in the buffer, or an error.
    pub async fn get_buffer_free(&self) -> Result<u16, CommandError> {
        let response = self
            .send_command(Command::GetRingbufferEmptySampleCount)
            .await?;
        match response {
            Response::BufferFree(free) => Ok(free),
            _ => unreachable!(),
        }
    }

    /// Enable or disable laser output.
    pub async fn set_output(&self, enable: bool) -> Result<(), CommandError> {
        let response = self.send_command(Command::SetOutput(enable)).await?;
        match response {
            Response::Ack => Ok(()),
            _ => unreachable!(),
        }
    }

    /// Enable or disable buffer size responses on data packets.
    pub async fn enable_buffer_size_response(&self, enable: bool) -> Result<(), CommandError> {
        let response = self
            .send_command(Command::EnableBufferSizeResponseOnData(enable))
            .await?;
        match response {
            Response::Ack => Ok(()),
            _ => unreachable!(),
        }
    }
}
