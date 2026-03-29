# Open GFX Board — Project Overview

## Summary

Open GFX Board is a desktop application for managing graphics overlays in OBS. It allows operators to trigger full-screen graphics (static images or looping videos) from a control interface, either from the host machine or any device on the same local network. It is designed for organizations running multiple simultaneous production studios.

## Technology Stack


| Layer                   | Technology                 |
| ----------------------- | -------------------------- |
| Desktop shell           | Tauri (Rust)               |
| Frontend (shared)       | Svelte + Vite              |
| Embedded HTTP/WS server | Axum                       |
| Real-time communication | socketioxide (SocketIO v5) |
| Key simulation          | enigo                      |
| Database                | SQLite                     |


## Architecture

The application is a Tauri desktop app. Inside the Rust backend, an Axum HTTP server runs on `0.0.0.0:PORT`, making it reachable by any device on the local network. This server handles three responsibilities:

- Serving the built Svelte frontend for external browser access
- Serving the OBS overlay pages
- Running the SocketIO server for real-time communication between clients and server

The Svelte frontend is shared between the Tauri app window and the network-served control interface. A `bridge.ts` abstraction layer routes calls to either Tauri's `invoke()` IPC (when running inside the Tauri WebView) or plain HTTP/WebSocket (when accessed from a remote device on the LAN).

```text
┌──────────────────────────────────────────────────────┐
│  Vite Build Output (/dist) — shared Svelte SPA       │
└──────┬───────────────────────────────────────────────┘
       │ embedded in binary          │ loaded by Tauri WebView
       ▼                             ▼
┌─────────────────┐         ┌────────────────────────┐
│  Axum :PORT     │         │  Tauri App Window      │
│  - serves /dist │         │  invoke() → Rust       │
│  - /obs         │         └────────────────────────┘
│  - SocketIO     │
└────────┬────────┘
         │ http://192.168.x.x:PORT
         ▼
  Any device on the same LAN
  (phones, tablets, other PCs)
         │
         ▼
┌────────────────┐
│  OBS Browser   │
│  Controler     │
│  :PORT/control │
└────────────────┘
```

## Studios

A studio corresponds to a single OBS instance running in a physical production room with its own cameras and setup. Each studio is represented as a SocketIO room. Clients (control interfaces and OBS overlays) join a specific studio room and only receive events scoped to that room. Multiple studios can be active simultaneously.

## Graphics

Currently supported graphic types:

- Static images (full-screen)
- Looping video files (full-screen)

Planned future support:

- HTML-based interactive sources (e.g. scoreboards, lower thirds)

Graphics are displayed via an OBS Browser Source pointed at the overlay page served by the embedded Axum server.

## OBS Integration

### Overlay

OBS consumes the overlay via a Browser Source. The overlay connects to the SocketIO server, joins its studio room, and reacts to graphic events in real time.

### Hotkey Triggering

The application simulates F13–F24 keypresses using the `enigo` Rust crate. These keys are mapped to actions in OBS's Hotkeys panel by the user. This allows Open GFX Board to trigger arbitrary OBS actions (scene switches, source toggles, etc.) without requiring a direct OBS API integration.

## Control Interface

- Served over HTTP by the embedded Axum server at `http://0.0.0.0:PORT`
- Accessible from the host machine or any device on the same local network
- No authentication — relies on the organization's network being properly managed
- The same Svelte build used by the Tauri app window is served to remote clients

## Persistence

All application state is stored in a SQL database. This includes studio configurations, graphic assets metadata, and any other user-defined settings.

## `bridge.ts` — Environment Abstraction

All communication between the Svelte frontend and the backend goes through `src/lib/bridge.ts`. This module detects whether it is running inside the Tauri WebView or a regular browser and routes calls accordingly:

- **Inside Tauri:** uses `invoke()` from `@tauri-apps/api/core`
- **In browser (LAN client):** uses `fetch` and native WebSocket

This allows all Svelte components to remain environment-agnostic.

## Platform Support


| Platform | Status         |
| -------- | -------------- |
| Windows  | Primary target |
| macOS    | Planned        |
| Linux    | Planned        |


