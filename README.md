# rKNX - KNX Control Daemon and Client in Rust

This project is a high-performance KNX control daemon and library written in Rust. It is based on the architecture and logic of **[KNX.ts](https://github.com/Wesxt/KNX.ts)** (the TypeScript KNX client and server), porting its robust FSM handling, ACK timeout controls (`ignoreACKTimeout`), and hardware adapters (USB, TPUART, IP Tunneling, IP Router, and IP Server).

Additionally, rKNX integrates a local SQLite database and exposes **WebSocket** and **MQTT** API servers to control and monitor the KNX bus asynchronously without blocking main telemetry threads.

---

## API Features

The API enables managing connection states, configuring Datapoint Types (DPTs), subscribing to group address updates, and sending group write and read commands.

### Connection Workflow
> [!IMPORTANT]
> A backend connection must be established via the API before executing other functions that interact with the KNX bus. The connection settings are persisted in the SQLite database and restored automatically upon daemon startup.

---

## 1. WebSocket API

By default, the WebSocket server listens on port `8080` (configurable via `api.ws_port` in `config.toml`). All requests and responses are framed in JSON.

### WebSocket Action Details

Below is the exact JSON structure for every action request and its corresponding successful `response` object.

#### 1. `connect`
Establishes a connection to the KNX bus.
- **Request**:
  ```json
  {
    "id": "1",
    "action": "connect",
    "connection_type": "Tunneling",
    "options": {
      "gateway_host": "192.168.1.10",
      "gateway_port": 3671,
      "transport": "Udp",
      "connection_type": "TunnelConnection",
      "auto_reconnect": true
    }
  }
  ```
  *(Supported `connection_type` values: `"Router"`, `"Server"`, `"Tunneling"`, `"Usb"`, `"Tpuart"`. The `options` object mirrors their respective sections from `config.toml`)*
- **Response**:
  ```json
  {
    "id": "1",
    "success": true,
    "response": { "connected": true }
  }
  ```

#### 2. `disconnect`
Closes the active connection.
- **Request**:
  ```json
  {
    "id": "2",
    "action": "disconnect"
  }
  ```
- **Response**:
  ```json
  {
    "id": "2",
    "success": true,
    "response": { "disconnected": true }
  }
  ```

#### 3. `subscribe`
Subscribes to real-time events for a group address.
- **Request**:
  ```json
  {
    "id": "3",
    "action": "subscribe",
    "group_address": "1/1/1"
  }
  ```
- **Response**:
  ```json
  {
    "id": "3",
    "success": true,
    "response": { "subscribed": "1/1/1" }
  }
  ```

#### 4. `unsubscribe`
Removes a group address subscription.
- **Request**:
  ```json
  {
    "id": "4",
    "action": "unsubscribe",
    "group_address": "1/1/1"
  }
  ```
- **Response**:
  ```json
  {
    "id": "4",
    "success": true,
    "response": { "unsubscribed": "1/1/1" }
  }
  ```

#### 5. `set_dpt`
Associates a Datapoint Type (DPT) with a group address to decode/encode values.
- **Request**:
  ```json
  {
    "id": "5",
    "action": "set_dpt",
    "group_address": "1/1/1",
    "dpt": "1.001"
  }
  ```
- **Response**:
  ```json
  {
    "id": "5",
    "success": true,
    "response": { "configured": "1/1/1", "dpt": "1.001" }
  }
  ```

#### 6. `write`
Writes a value to a group address on the KNX bus. Requires a DPT to have been configured for the address first. The `value` field must match the specific structured DPT interface (fully compatible with `DPTs.ts`).

- **Requests (Examples based on DPT type)**:
  - **DPT 1 (Switching/Boolean)**:
    ```json
    {
      "id": "6a",
      "action": "write",
      "group_address": "1/1/1",
      "value": { "value": true }
    }
    ```
    *(For backwards compatibility, passing raw value directly e.g. `"value": true` is also supported)*
  
  - **DPT 3 (Control Dimming/Blinds)**:
    ```json
    {
      "id": "6b",
      "action": "write",
      "group_address": "1/1/2",
      "value": { "control": 1, "stepCode": 5 }
    }
    ```
  
  - **DPT 232 (RGB Color)**:
    ```json
    {
      "id": "6c",
      "action": "write",
      "group_address": "1/1/3",
      "value": { "R": 255, "G": 0, "B": 128 }
    }
    ```

- **Response**:
  ```json
  {
    "id": "6",
    "success": true,
    "response": { "written": true }
  }
  ```

#### 7. `read`
Sends a group value read request (`AGroupValueRead`) to query the state of a group address.
- **Request**:
  ```json
  {
    "id": "7",
    "action": "read",
    "group_address": "1/1/1"
  }
  ```
- **Response**:
  ```json
  {
    "id": "7",
    "success": true,
    "response": { "read_sent": true }
  }
  ```

#### 8. `status`
Retrieves the current daemon runtime information.
- **Request**:
  ```json
  {
    "id": "8",
    "action": "status"
  }
  ```
- **Response**:
  ```json
  {
    "id": "8",
    "success": true,
    "response": {
      "connected": true,
      "connection_type": "Tunneling",
      "individual_address": "1.1.255",
      "subscriptions": ["1/1/1", "1/1/2"],
      "retention_seconds": 604800
    }
  }
  ```

#### 9. `get_history`
Retrieves past logged indications from the SQLite database.
- **Request**:
  ```json
  {
    "id": "9",
    "action": "get_history",
    "limit": 2
  }
  ```
- **Response**:
  ```json
  {
    "id": "9",
    "success": true,
    "response": [
      {
        "id": 15,
        "timestamp": 1720310400,
        "group_address": "1/1/1",
        "cemi_hex": "2900bce0000011010100",
        "description": {
          "obj": "L_Data_ind",
          "message_code": 41,
          "source_address": "1.1.10",
          "destination_address": "1/1/1",
          "control_field1": {
            "obj": "ControlField",
            "hex": "0xbc",
            "frame_type": "Data",
            "priority": "Low",
            "confirm": "NoConfirm"
          }
        },
        "value": true
      }
    ]
  }
  ```

#### 10. `set_retention`
Updates the automatic telemetry database retention period.
- **Request**:
  ```json
  {
    "id": "10",
    "action": "set_retention",
    "seconds": 86400
  }
  ```
- **Response**:
  ```json
  {
    "id": "10",
    "success": true,
    "response": { "retention_configured": 86400 }
  }
  ```

---

### Real-time Event Broadcaster (WebSocket Broadcast)

When a group address that has an active subscription receives a packet, an event is broadcast to all connected WebSocket clients:

```json
{
  "event": "indication",
  "group_address": "1/1/1",
  "timestamp": 1720310400,
  "description": {
    "obj": "L_Data_ind",
    "message_code": 41,
    "source_address": "1.1.10",
    "destination_address": "1/1/1",
    "control_field1": {
      "obj": "ControlField",
      "hex": "0xbc",
      "frame_type": "Data",
      "priority": "Low",
      "confirm": "NoConfirm"
    }
  },
  "value": true
}
```

---

## 2. MQTT API

If enabled in `config.toml`, rKNX connects to an MQTT broker and publishes/subscribes to the following topics:

### Command Topics (`rknx/cmd/<action>`)

To trigger commands, publish a JSON payload on the respective action topic. Command results are published back on `rknx/response/<action>`.

- **Connect**: `rknx/cmd/connect`
  ```json
  {
    "connection_type": "Tunneling",
    "options": {
      "gateway_host": "192.168.1.10",
      "gateway_port": 3671
    }
  }
  ```
- **Disconnect**: `rknx/cmd/disconnect`
  *(Empty payload or any JSON)*
- **Subscribe**: `rknx/cmd/subscribe`
  ```json
  { "group_address": "1/1/1" }
  ```
  *(Or raw string `"1/1/1"`)*
- **Unsubscribe**: `rknx/cmd/unsubscribe`
  ```json
  { "group_address": "1/1/1" }
  ```
- **Configure DPT**: `rknx/cmd/set_dpt`
  ```json
  {
    "group_address": "1/1/1",
    "dpt": "1.001"
  }
  ```
- **Write Value**: `rknx/cmd/write`
  ```json
  {
    "group_address": "1/1/1",
    "value": true
  }
  ```
- **Read Value**: `rknx/cmd/read`
  ```json
  { "group_address": "1/1/1" }
  ```
- **Set Telemetry Retention**: `rknx/cmd/set_retention`
  ```json
  { "seconds": 86400 }
  ```

### Response Topics (`rknx/response/<action>`)

Response payloads are formatted as JSON:
```json
{
  "success": true,
  "response": { "connected": true }
}
```

### Event & Telemetry Topics

- **Daemon Status**: `rknx/status`
  - Publishes `online` on daemon startup.
  - Publishes `offline` gracefully or via Last Will and Testament (LWT).
- **Indication Log Detail**: `rknx/event/indication/<group_address>`
  - Publishes structured JSON detailing the parsed CEMI representation (`cemi.describe`), timestamp, and decoded value:
  ```json
  {
    "event": "indication",
    "group_address": "1/1/1",
    "timestamp": 1720310400,
    "description": {
      "obj": "L_Data_ind",
      "message_code": 41,
      "source_address": "1.1.10",
      "destination_address": "1/1/1",
      "control_field1": {
        "obj": "ControlField",
        "hex": "0xbc",
        "frame_type": "Data",
        "priority": "Low",
        "confirm": "NoConfirm"
      }
    },
    "value": true
  }
  ```
- **Group Address Value State**: `rknx/event/state/<group_address>`
  - Publishes the plain decoded value directly as a string (e.g. `"true"` or `"128"`) for quick and straightforward integrations (e.g., Home Assistant).

---

## SQLite Database Schema & Persistence

To maintain daemon state across service restarts, the following configurations are saved and loaded transparently from `rknx_cache.db`:

1. **Active Connection Settings**: The connection type and JSON options parameters, along with the connected state.
2. **Subscriptions**: The list of all subscribed group addresses.
3. **Datapoint Type (DPT) Configuration**: Association mappings between group addresses and DPT strings.
4. **Log Retention (TTL)**: Telemetry records age retention settings. A background job handles periodic garbage collection.
5. **Indication History**: A non-blocking telemetry frame database log table (`indications_history`) is populated asynchronously from the cache notifier.
