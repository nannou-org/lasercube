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

    // Set a timeout for discovery
    let discovery = timeout(Duration::from_secs(5), async {
        while let Some(device_info) = devices.next().await {
            println!("Found LaserCube: {device_info:#?}");
        }
    });

    // Wait for timeout or completion
    match discovery.await {
        Ok(_) => println!("Discovery complete"),
        Err(_) => println!("Discovery timeout"),
    }

    Ok(())
}
