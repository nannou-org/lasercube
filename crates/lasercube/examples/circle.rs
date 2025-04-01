use futures::StreamExt;
use lasercube::core::{Command, Point, SampleData, MAX_POINTS_PER_MESSAGE};
use lasercube::Client;
use lasercube_core::cmds::Response;
use std::f32::consts::PI;
use std::time::Duration;
use tokio::time::timeout;

/// Generate a point at a specific position on a circle
fn circle_point(index: usize, total_points: usize, radius: f32) -> Point {
    // Calculate angle based on index and total points (distributing points evenly)
    let angle = 2.0 * PI * (index as f32) / (total_points as f32);

    // Calculate x,y coordinates
    let x = radius * angle.cos();
    let y = radius * angle.sin();

    // Create a point with white color
    Point::from_normalized([x, y], [1.0, 1.0, 1.0])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable logging
    let _ = tracing_subscriber::fmt::try_init();

    // Begin discovery
    let bind_ip = [0, 0, 0, 0].into();
    let target_ip = [255, 255, 255, 255].into();
    let mut devices = lasercube::discover::devices(bind_ip, target_ip).await?;

    tracing::info!("Discovering devices for 5 seconds");

    // Set a timeout for discovery
    let device_info = timeout(Duration::from_secs(5), async {
        devices.next().await.expect("No LaserCube devices found")
    })
    .await
    .expect("Failed to find a LaserCube device");

    tracing::info!("Found LaserCube: {:#?}", device_info);

    // Connect to the discovered device
    let client = Client::new(bind_ip, device_info.header.ip_addr).await?;

    // Create a socket for the DATA port
    let data_socket = tokio::net::UdpSocket::bind((bind_ip, 0)).await?;
    let data_addr =
        std::net::SocketAddrV4::new(device_info.header.ip_addr, lasercube::core::port::DATA);

    // Enable buffer size responses, so we know when we can send more data
    tracing::debug!("Enabling buffer size responses");
    client.enable_buffer_size_response(true).await?;

    // Enable laser output
    client.set_output(true).await?;
    tracing::info!("Laser output enabled");

    // Circle configuration
    let total_points = MAX_POINTS_PER_MESSAGE;
    let radius = 0.8;

    // Buffer for receiving responses
    let mut response_buf = vec![0u8; 1024];

    // Message and frame counters
    let mut message_num = 0u8;
    let mut frame_num = 0u8;

    // Index tracking for continuous circle
    let mut current_index = 0;

    // Track buffer free space, based on the latency we want.
    const MAX_LATENCY_MS: u16 = 64;
    let max_buffer_points = (device_info.header.dac_rate / 1_000) as u16 * MAX_LATENCY_MS;
    let max_buffer_free = device_info.header.rx_buffer_size.min(max_buffer_points);
    let buffer_free_diff = device_info.header.rx_buffer_size - max_buffer_free;
    let mut buffer_free = device_info
        .header
        .rx_buffer_free
        .saturating_sub(buffer_free_diff);

    tracing::info!("Starting to stream circle pattern...");
    tracing::info!("Press Ctrl+C to exit");

    loop {
        tracing::debug!("buffer_free: {buffer_free} | msg: {message_num}");

        // Calculate how many points we can send based on available buffer space
        // but limit to a reasonable number to avoid overly large packets
        let points_to_send = (buffer_free as usize).min(MAX_POINTS_PER_MESSAGE);

        // Create a batch of points starting from current_index
        let mut batch_points = Vec::with_capacity(points_to_send);
        for _ in 0..points_to_send {
            batch_points.push(circle_point(current_index, total_points, radius));
            current_index = (current_index + 1) % total_points;
            // At the end of each complete circle, increment the frame number
            if current_index == 0 {
                frame_num = frame_num.wrapping_add(1);
            }
        }

        // Create and send the sample data
        let sample_data = SampleData {
            message_num,
            frame_num,
            points: batch_points,
        };

        let command = Command::SampleData(sample_data);
        let bytes = command.to_bytes();
        data_socket.send_to(&bytes, data_addr).await?;

        // Update tracking
        message_num = message_num.wrapping_add(1);

        // Deduct points from buffer (will be updated when response received)
        let points_sent = points_to_send as u16;
        buffer_free = buffer_free.saturating_sub(points_sent);

        // Wait for buffer feedback with a short timeout
        // This ensures we get an accurate buffer state without blocking too long
        match timeout(
            Duration::from_millis(10),
            data_socket.recv_from(&mut response_buf),
        )
        .await
        {
            Ok(Ok((len, _addr))) => {
                let res = Response::try_from(&response_buf[0..len]);
                tracing::debug!("response: {res:?}");
                match res {
                    Ok(Response::BufferFree(free)) => {
                        buffer_free = free.saturating_sub(buffer_free_diff);
                    }
                    Ok(response) => {
                        tracing::error!("Unexpected response: {response:?}");
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse response: {e}");
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::error!("Error receiving buffer response: {}", e);
            }
            Err(_) => {
                // Timeout occurred, continue with current buffer estimate
                tracing::debug!("Response timeout, using estimated buffer: {}", buffer_free);
            }
        }
    }
}
