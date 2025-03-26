use futures::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable logging.
    let _ = tracing_subscriber::fmt::try_init();

    // Begin discovery.
    let bind_ip = [0, 0, 0, 0].into();
    let target_ip = [255, 255, 255, 255].into();
    let mut devices = lasercube::discover::devices(bind_ip, target_ip).await?;

    tracing::info!("Discovering devices for 5 seconds");

    // Set a timeout for discovery
    let discovery = timeout(Duration::from_secs(5), async {
        while let Some(device_info) = devices.next().await {
            tracing::info!("Found LaserCube: {device_info:#?}");
        }
    });

    // Wait for timeout or completion
    let _ = discovery.await;
    tracing::info!("Discovery complete");

    Ok(())
}
