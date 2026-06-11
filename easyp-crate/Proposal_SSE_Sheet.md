# SSE Sheet — Proposal

## What

A multi-user editable spreadsheet backed directly by the server's log capture system and SSE broadcast infrastructure. Each row is a log entry; each column is an editable field (timestamp, level, message, source, plus a mutable note/annotation column).

## Architecture

Same bidirectional pattern as the logwatcher: SSE for pushes, HTTP POST for writes.

```
┌─────────┐   POST /logs_KEY {edit}    ┌──────────────┐
│ Client A │ ──────────────────────────→ │              │
│          │                             │  LogStorage  │
│ Client B │ ←─── SSE: data: {diff} ──── │ + CellStore  │
└─────────┘                             └──────────────┘
                                               │
                                     broadcast_log_entry()
                                               │
                                          ┌────┴────┐
                                          │ SSE subs │
                                          └─────────┘
```

## Data model

### LogStorage (existing, extended)

Each `LogEntry` gains an optional `annotations: HashMap<String, String>` keyed by column name (e.g. `"note"`, `"assignee"`, `"status"`).

### CellStore (new)

A lightweight in-memory store for cell-level metadata not tied to log entries — e.g. column headers, frozen columns, filter state, per-user cursor positions.

```rust
struct CellStore {
    /// Per-column metadata (header label, width, frozen)
    columns: Vec<ColumnMeta>,
    /// Per-cell annotations keyed by (row_idx, col_name)
    annotations: HashMap<(usize, String), String>,
    /// Active users for presence
    presence: HashMap<String, UserCursor>,
}
```

## Protocol

### Client → Server (POST)

```
POST /logs_KEY
Content-Type: application/json
Body: {
  "action": "edit_cell",
  "row": 42,
  "col": "note",
  "value": "Needs investigation"
}
```

Additional actions: `resize_column`, `add_column`, `freeze_row`, `presence` (cursor position).

### Server → Client (SSE)

Existing SSE endpoint at `/logs_KEY?sse=1`. Event payload is JSON:

```
data: {"type":"cell_edit","row":42,"col":"note","value":"Needs investigation","by":"user_alice"}
data: {"type":"presence","user":"user_bob","row":38,"col":"message","color":"#4ec9b0"}
data: {"type":"column_resize","col":"message","width":300}
```

The existing `broadcast_log_entry()` infrastructure handles delivery to all connected clients. Events are typed so the client dispatches to the right handler (render cell, paint cursor, resize column).

## Client rendering

A single-page HTML table rendered by the admin panel. Uses the same `EventSource` connection as the log viewer.

```javascript
const es = new EventSource(window.location.pathname + '?sse=1');
es.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  switch (msg.type) {
    case 'cell_edit': updateCell(msg.row, msg.col, msg.value); break;
    case 'presence':  paintCursor(msg.user, msg.row, msg.col, msg.color); break;
    case 'log_entry': prependRow(msg.timestamp, msg.level, msg.message); break;
  }
};
```

## Implementation plan

| Step | Work |
|------|------|
| 1 | Extend `LogEntry` / `LogStorage` with annotations map |
| 2 | Add `CellStore` struct (shared globals like `LOG_SUBSCRIBERS`) |
| 3 | Add POST handlers in `handle_logs_admin_request` for `edit_cell` etc. |
| 4 | Add SSE event types for cell edits, column changes, presence |
| 5 | Build the HTML/JS spreadsheet UI (table with editable cells) |
| 6 | Integrate with existing admin page as a toggle ("Sheet view" / "Log view") |

No changes needed to the transport layer — SSE + POST handles it. The broadcast infrastructure from the logwatcher (`LOG_SUBSCRIBERS`, `broadcast_log_entry()`) is reused directly for cell edits and presence.

## Why not WebSocket

- SSE is already implemented, tested, and in production
- Cell edits are discrete save-on-blur events, not keystroke streams
- HTTP POST for writes is simpler to debug, proxy, and extend
- Presence updates (cursor position) are low-frequency enough for SSE
- Zero additional dependencies
