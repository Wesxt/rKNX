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

### Request Payload Format

```json
{
  "id": "unique_message_id",
  "action": "action_name",
  "connection_type": "Optional (Router | Server | Tunneling | Usb | Tpuart)",
  "options": { ... "action_parameters" ... },
  "group_address": "Optional (e.g., 1/1/1)",
  "dpt": "Optional (e.g., 1.001)",
  "value": "Optional (value to write)"
}
```

### Response Payload Format

- **Success**:
  ```json
  {
    "id": "unique_message_id",
    "success": true,
    "response": { ... "return_data" ... }
  }
  ```
- **Error**:
  ```json
  {
    "id": "unique_message_id",
    "success": false,
    "error": "Error description message"
  }
  ```

### Supported Actions

| Action | Description | Key Parameters |
| :--- | :--- | :--- |
| `connect` | Connects to the bus using a specified backend connection. | `connection_type`, `options` |
| `disconnect` | Disconnects the active backend connection. | None |
| `subscribe` | Subscribes to real-time events for a group address. | `group_address` |
| `unsubscribe` | Unsubscribes from events for a group address. | `group_address` |
| `set_dpt` | Configures the Datapoint Type (DPT) for a group address. | `group_address`, `dpt` |
| `write` | Sends a group value write request (`AGroupValueWrite`). | `group_address`, `value` |
| `read` | Sends a group value read request (`AGroupValueRead`). | `group_address` |
| `status` | Retrieves daemon status, active connections, and subscriptions. | None |
| `get_history` | Queries the DB indication logs cache. | `limit` (default is 50) |
| `set_retention` | Sets the maximum telemetry retention limit in seconds. | `seconds` |

---

### Real-time Event Broadcaster (WebSocket Broadcast)

When a group address that has an active subscription receives a packet, an event is broadcast to all connected WebSocket clients:

```json
{
  "event": "indication",
  "group_address": "1/1/1",
  "timestamp": 1720310400,
  "description": "CEMI {\n  obj: 'L_Data_ind',\n  messageCode: 41,\n  ...",
  "value": "Some(Dpt1(true))"
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
    "description": "CEMI {\n  obj: 'L_Data_ind', ...",
    "value": "Some(Dpt1(true))"
  }
  ```
- **Group Address Value State**: `rknx/event/state/<group_address>`
  - Publishes the plain decoded value directly as a string (e.g. `"Dpt1(true)"` or `"Dpt5(128)"`) for quick and straightforward integrations (e.g., Home Assistant).

---

## SQLite Database Schema & Persistence

To maintain daemon state across service restarts, the following configurations are saved and loaded transparently from `rknx_cache.db`:

1. **Active Connection Settings**: The connection type and JSON options parameters, along with the connected state.
2. **Subscriptions**: The list of all subscribed group addresses.
3. **Datapoint Type (DPT) Configuration**: Association mappings between group addresses and DPT strings.
4. **Log Retention (TTL)**: Telemetry records age retention settings. A background job handles periodic garbage collection.
5. **Indication History**: A non-blocking telemetry frame database log table (`indications_history`) is populated asynchronously from the cache notifier.
