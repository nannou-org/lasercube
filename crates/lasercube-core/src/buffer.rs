//! Buffer management for LaserCube devices.

/// Default buffer size from observed devices.
pub const DEFAULT_SIZE: u16 = 6_000;
/// Recommended buffer threshold for maintaining stability vs latency
pub const DEFAULT_THRESHOLD: u16 = 5_000;

/// Tracks the state of the LaserCube's buffer.
#[derive(Debug, Clone, Copy)]
pub struct BufferState {
    /// Total buffer size.
    pub total_size: u16,
    /// Current free space in the buffer.
    pub free_space: u16,
    /// Threshold for deciding when to send more data.
    pub threshold: u16,
    /// Last time we received a buffer update (in milliseconds since start).
    pub last_update_time: u64,
}

impl BufferState {
    pub const DEFAULT: Self = Self {
        total_size: DEFAULT_SIZE,
        free_space: DEFAULT_SIZE,
        threshold: DEFAULT_THRESHOLD,
        last_update_time: 0,
    };

    /// Create a new `BufferState` with default values.
    pub fn new() -> Self {
        Self::DEFAULT
    }

    /// Update buffer free space from device response.
    pub fn update_free_space(&mut self, free_space: u16, current_time: u64) {
        self.free_space = free_space;
        self.last_update_time = current_time;
    }

    /// Update total buffer size from device response.
    pub fn update_total_size(&mut self, total_size: u16) {
        self.total_size = total_size;

        // Adjust threshold to be a percentage of total size
        // Maintain latency vs stability tradeoff
        if total_size > 1000 {
            self.threshold = total_size - 1000;
        } else {
            // Fallback for very small buffers
            self.threshold = total_size / 6 * 5;
        }
    }

    /// Check if we should send more data based on buffer free space.
    pub fn should_send(&self) -> bool {
        self.free_space >= self.threshold
    }

    /// Estimate current free space based on time elapsed and DAC rate.
    pub fn estimate_current_free_space(&self, current_time: u64, dac_rate: u32) -> u16 {
        if dac_rate == 0 || self.last_update_time == 0 {
            return self.free_space;
        }

        // Calculate time delta in milliseconds
        let delta_ms = if current_time > self.last_update_time {
            current_time - self.last_update_time
        } else {
            // Handle possible timer wraparound
            0
        };

        // Convert from DAC rate (points per second) to points per millisecond
        let points_per_ms = dac_rate as f32 / 1000.0;

        // Calculate estimated points consumed
        let points_consumed = (delta_ms as f32 * points_per_ms) as u16;

        // Add to free space, but don't exceed total buffer size
        let estimated_free = self
            .free_space
            .saturating_add(points_consumed)
            .min(self.total_size);

        estimated_free
    }

    /// Update the buffer when points are sent.
    pub fn consume(&mut self, points_sent: u16) {
        self.free_space = self.free_space.saturating_sub(points_sent);
    }
}

impl Default for BufferState {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_defaults() {
        let buffer = BufferState::new();

        assert_eq!(buffer.total_size, DEFAULT_SIZE);
        assert_eq!(buffer.free_space, DEFAULT_SIZE);
        assert_eq!(buffer.threshold, DEFAULT_THRESHOLD);
        assert_eq!(buffer.last_update_time, 0);
    }

    #[test]
    fn test_update_free_space() {
        let mut buffer = BufferState::new();

        // Test updating free space
        buffer.update_free_space(3000, 100);
        assert_eq!(buffer.free_space, 3000);
        assert_eq!(buffer.last_update_time, 100);

        // Test updating free space again
        buffer.update_free_space(4000, 200);
        assert_eq!(buffer.free_space, 4000);
        assert_eq!(buffer.last_update_time, 200);
    }

    #[test]
    fn test_update_total_size() {
        let mut buffer = BufferState::new();

        // Test with normal buffer size
        buffer.update_total_size(8000);
        assert_eq!(buffer.total_size, 8000);
        assert_eq!(buffer.threshold, 7000); // 8000 - 1000

        // Test with small buffer size
        buffer.update_total_size(600);
        assert_eq!(buffer.total_size, 600);
        assert_eq!(buffer.threshold, 500); // 600 / 6 * 5
    }

    #[test]
    fn test_should_send() {
        let mut buffer = BufferState::new();
        buffer.threshold = 4000;

        // Test when free space is below threshold
        buffer.free_space = 3999;
        assert!(!buffer.should_send());

        // Test when free space is at threshold
        buffer.free_space = 4000;
        assert!(buffer.should_send());

        // Test when free space is above threshold
        buffer.free_space = 4001;
        assert!(buffer.should_send());
    }

    #[test]
    fn test_estimate_current_free_space() {
        let mut buffer = BufferState::new();
        buffer.total_size = 6000;
        buffer.free_space = 3000;
        buffer.last_update_time = 1000;

        // Test with zero DAC rate
        let estimate = buffer.estimate_current_free_space(2000, 0);
        assert_eq!(estimate, 3000); // Should remain unchanged

        // Test with non-zero DAC rate (1000 points per second)
        // 1000 ms elapsed, 1000 points per second = 1000 points
        let estimate = buffer.estimate_current_free_space(2000, 1000);
        assert_eq!(estimate, 4000); // 3000 + 1000

        // Test that estimate doesn't exceed total size
        buffer.free_space = 5500;
        let estimate = buffer.estimate_current_free_space(2000, 1000);
        assert_eq!(estimate, 6000); // Capped at total_size

        // Test with time wraparound (current time < last update time)
        buffer.free_space = 3000;
        buffer.last_update_time = 2000;
        let estimate = buffer.estimate_current_free_space(1000, 1000);
        assert_eq!(estimate, 3000); // Should remain unchanged
    }

    #[test]
    fn test_consume() {
        let mut buffer = BufferState::new();
        buffer.free_space = 5000;

        // Test normal consumption
        buffer.consume(1000);
        assert_eq!(buffer.free_space, 4000);

        // Test consumption that would go below zero
        buffer.consume(5000);
        assert_eq!(buffer.free_space, 0); // Should saturate at 0
    }

    #[test]
    fn test_integrated_buffer_scenario() {
        // Simulating a realistic usage scenario
        let mut buffer = BufferState::new();

        // Initialize with device info
        buffer.update_total_size(6000);
        buffer.update_free_space(6000, 100);

        // Send some points
        buffer.consume(1000);
        assert_eq!(buffer.free_space, 5000);

        // Device renders some points over time
        // 500ms passes, DAC rate is 1000 points/sec
        let estimate = buffer.estimate_current_free_space(600, 1000);
        assert_eq!(estimate, 5500); // 5000 + (500 * 1000 / 1000)

        // Update with actual device reported free space
        buffer.update_free_space(5400, 600); // Maybe some overhead in actual device

        // Send more points
        buffer.consume(2000);
        assert_eq!(buffer.free_space, 3400);

        // Check if we should send more
        assert!(!buffer.should_send()); // 3400 < 5000 threshold
    }
}
