
# NOTE

This document reflects a previous iteration and is partially outdated. The current direction is an ECS-first, headless core with broadcast events and optional UI via egui_tiles. See `PLANS.md` at the repository root for the up-to-date architecture plan and roadmap.

# Lunatic Studio Plugin Development Guide

Welcome to Lunatic Studio's plugin development framework. This guide is intended for developers who want to create plugins for Lunatic Studio using its command-driven, mailbox-based architecture.

---

## 📦 Architecture Overview

Lunatic Studio uses a modular, asynchronous messaging system centered around a `Mailbox`. All subsystems—including the renderer, UI, plugin manager, and user plugins—communicate via message envelopes sent through this central hub.

- **CORE**: The rendering engine. It consumes render commands and outputs frames to a shared buffer.
- **UI**: Communicates via the mailbox and listens/responds to UI-related messages.
- **Plugins**: Dynamically loaded components that send and receive commands.
- **Mailbox (Kernel)**: The routing center for all messages. It dispatches them to appropriate handlers.

---

## 📬 Message Protocol

All communication is done through messages sent to the `Mailbox`. Each message follows this general structure:

```rust
#[repr(C)]
pub struct Envelope {
    pub opcode: u32,            // Operation code
    pub source: u32,            // Unique ID of the sender
    pub destination: u32,       // Target subsystem or plugin
    pub payload: *const u8,     // Pointer to payload bytes
    pub length: usize,          // Size of payload
}
```

### Example

A plugin might send:

```rust
Envelope {
    opcode: 0x1002, // RENDER_FRAME
    source: 0x02,   // Plugin ID
    destination: 0x01, // CORE
    payload: ptr::to_frame_data(),
    length: frame_size,
}
```

---

## 🧩 Plugin Lifecycle

1. **Loading**  
   The plugin is dynamically loaded (as `.so`/`.dll`) by the plugin manager.

2. **Initialization**  
   Your plugin must export an `extern "C"` function:

   ```c
   void lunatic_plugin_init(LunaticContext* ctx);
   ```

   `LunaticContext` contains API pointers for sending messages, allocating memory, and logging.

3. **Message Handling**  
   You must register a callback function to receive messages:

   ```c
   void lunatic_plugin_receive(const Envelope* msg);
   ```

---

## ⚙ OpCodes

Each message must include an `opcode`, a `u32` that indicates the type of operation. These are currently under development. Planned categories:

- `0x0000` — System (Init, Shutdown, Heartbeat)
- `0x1000` — Render-related
- `0x2000` — UI actions
- `0x3000` — Audio/MIDI
- `0x8000` — Plugin-defined/custom

Plugins are free to define opcodes in the `0x8000..0xFFFF` range.

---

## ✅ Best Practices

- Always check the `opcode` and `destination` before acting on a message.
- Keep message handlers non-blocking. If work is expensive, spawn a thread.
- Don’t panic inside your plugin. Return errors through messages.
- Use ACK messages to signal that you've completed or rejected a request.

---

## 🧪 Testing Your Plugin

You can run Lunatic Studio in CLI mode with:

```sh
lunatic-core --load-plugin path/to/your_plugin.dll
```

Use the `--debug` flag to log all message traffic.

---

## 📚 Coming Soon

- Complete list of system opcodes
- Message builder helpers
- Async-safe message queue API
- Plugin dependency support
- Plugin-to-plugin messaging

---

## 🧠 Need Help?

Feel free to contribute, open issues, or request features. This ecosystem is open and community-driven.

---
