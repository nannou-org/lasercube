# LaserCube Network Protocol Specification

This document outlines the network protocol used to control LaserCube devices over UDP, based on reverse engineering efforts from the original Python implementation.

## Overview

The LaserCube device listens on multiple UDP ports and accepts commands for device control, status querying, and laser point rendering. The protocol can address individual LaserCubes via unicast or multiple devices simultaneously via broadcast messages.

## Network Configuration

### Ports

The device listens on three primary UDP ports:

| Port | Name | Purpose |
|------|------|---------|
| 45456 | ALIVE_PORT | For "alive" messages (simple pings to check which lasers are on the network) |
| 45457 | CMD_PORT | For commands (get information, enable/disable output, etc.) |
| 45458 | DATA_PORT | For sending point data to render with the laser |

### Addressing

* **Unicast**: Send commands to a specific LaserCube IP address
* **Broadcast**: Send to the broadcast address (255.255.255.255) to control all LaserCubes on the network simultaneously

## Message Format

All messages follow a common structure where the first byte is the command ID, followed by command-specific parameters. For data messages, care must be taken to keep total message size below the network MTU (typically 1500 bytes).

## Commands

### Device Discovery and Information

#### GET_FULL_INFO (0x77)
* **Port**: CMD_PORT (45457)
* **Direction**: Client → LaserCube
* **Format**: `[0x77]` (single byte)
* **Response**: Device returns detailed status information

**Response format (64 bytes total)**:
```
Offset  Size    Description
0       1       Command echo (0x77)
1       2       Padding
3       1       Firmware major version
4       1       Firmware minor version
5       1       Output enabled flag (boolean)
6       5       Padding
11      4       Current DAC rate (uint32, little-endian)
15      4       Maximum DAC rate (uint32, little-endian)
19      1       Padding
20      2       RX buffer free space (uint16, little-endian)
22      2       RX buffer size (uint16, little-endian)
24      1       Battery percentage
25      1       Temperature
26      6       Serial number (first byte doubles as connection type)
32      4       IP address (4 bytes)
36      1       Padding
37      1       Model number
38      ?       Model name (null-terminated string)
```

The model name field should not extend beyond 26 bytes (i.e. the 64th byte).

**Connection Types** (first byte of serial number at offset 26):
* 0: Unknown
* 1: USB
* 2: Ethernet
* 3: WiFi

### Buffer Management

#### ENABLE_BUFFER_SIZE_RESPONSE_ON_DATA (0x78)
* **Port**: CMD_PORT (45457)
* **Direction**: Client → LaserCube
* **Format**: `[0x78, enable]` where enable is 0x0 (disabled) or 0x1 (enabled)
* **Response**: Simple acknowledgment (`[0x78]`)
* **Purpose**: When enabled, the device will reply to data packets with buffer information

#### GET_RINGBUFFER_EMPTY_SAMPLE_COUNT (0x8a)
* **Port**: CMD_PORT (45457)
* **Direction**: Client → LaserCube
* **Format**: `[0x8a]` (single byte)
* **Response**: `[0x8a, padding, buffer_space_lo, buffer_space_hi]` (4 bytes)
* **Purpose**: Query how much free space is available in the device's buffer

**Response format**:
```
Offset  Size    Description
0       1       Command echo (0x8a)
1       1       Padding
2       2       Free buffer space (uint16, little-endian)
```

### Output Control

#### SET_OUTPUT (0x80)
* **Port**: CMD_PORT (45457)
* **Direction**: Client → LaserCube
* **Format**: `[0x80, enable]` where enable is 0x0 (disabled) or 0x1 (enabled)
* **Response**: Simple acknowledgment (`[0x80]`)
* **Purpose**: Enable or disable laser output

### Point Data Transmission

#### SAMPLE_DATA (0xa9)
* **Port**: DATA_PORT (45458)
* **Direction**: Client → LaserCube
* **Format**:
  ```
  [0xa9, 0x00, message_number, frame_number, point_data...]
  ```
  Where:
  - `message_number` is a sequence number (0-255) that increments with each message
  - `frame_number` is a sequence number (0-255) that increments with each complete frame
  - `point_data` is a sequence of point structures

  Each point is represented by 5 uint16 values packed as little-endian:
  ```
  [x_lo, x_hi, y_lo, y_hi, r_lo, r_hi, g_lo, g_hi, b_lo, b_hi]
  ```

  Where:
  - `x`, `y` are 12-bit coordinates (0x000-0xFFF range) centered at 0x800, 0x800
  - `r`, `g`, `b` are 12-bit color values (0x000-0xFFF range)

* **Response**: If ENABLE_BUFFER_SIZE_RESPONSE_ON_DATA is enabled, device replies with:
  ```
  [0xa9, buffer_free_lo, buffer_free_hi]
  ```
  Where `buffer_free` is the current free space in the device's buffer (uint16, little-endian)

## Implementation Details

### Point Coordinates and Colors

* Point coordinates are 12-bit values (0x000-0xFFF)
* The center of the coordinate system is at (0x800, 0x800)
* Color values are also 12-bit (0x000-0xFFF)

### Buffer Management Strategy

The LaserCube contains a ring buffer for storing point data before rendering. Effective management of this buffer is critical for smooth operation:

1. **Buffer Size Information**:
   - The buffer size is typically around 6000 points
   - Current free space can be queried with GET_RINGBUFFER_EMPTY_SAMPLE_COUNT
   - Buffer information is also returned in LaserInfo responses

2. **Flow Control**:
   - Enable buffer size responses on data packets using ENABLE_BUFFER_SIZE_RESPONSE_ON_DATA
   - Monitor the free space in the buffer
   - Adjust transmission rate based on available space
   - A good strategy is to wait when buffer free space drops below a threshold (typically ~5000)
   - Calculate estimated buffer consumption based on DAC rate and time elapsed

3. **Sequence Numbers**:
   - Increment the message_number with each SAMPLE_DATA message
   - Increment the frame_number when a complete frame of points is sent
   - Both sequence numbers wrap around to 0 after 255

### Message Transmission Reliability

For reliability in potentially lossy network environments:
1. Send critical commands twice (commands are designed to be idempotent)
2. Keep DATA messages under 1500 bytes (typical network MTU)
3. Limit to around 140 points per DATA message
4. Monitor buffer space to detect and recover from lost packets

### Startup Sequence

1. Discover devices with broadcast GET_FULL_INFO
2. Enable buffer size responses with ENABLE_BUFFER_SIZE_RESPONSE_ON_DATA
3. Enable output with SET_OUTPUT
4. Begin transmitting point data with SAMPLE_DATA
5. Monitor buffer capacity and adjust transmission rate

### Shutdown Sequence

1. Disable buffer size responses with ENABLE_BUFFER_SIZE_RESPONSE_ON_DATA
2. Disable output with SET_OUTPUT

## Safety Considerations

The LaserCube is a physical laser device that can present safety risks if misused. Implementations should:

1. Include safety timeouts that disable the laser if communication is lost
2. Provide emergency stop functionality
3. Respect the device's buffer management to prevent erratic behavior
4. Always test with safety lenses when developing new implementations
