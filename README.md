# Open GFX Board

**Control and manage your OBS graphics with a simple interface from your phone, tablet, or desktop.**

![Open GFX Board Interface](Open%20GFX%20Board.png)

Open GFX Board is a desktop application that puts powerful graphics control at your fingertips. Trigger full-screen images and videos in OBS instantly, from anywhere on your network. Perfect for live productions, gaming streams, sports broadcasts, and small broadcaster operations.

## ✨ Features

- **Remote Control**: Access a control pad from your phone, tablet, or any device on your network
- **Simplicity**: Trigger graphics with a single tap on a touch first interface (no fumbling through OBS scenes and sources)
- **Network Accessible**: No complicated setup just scan the generated QR Code and you are ready to control from any device
- **Open Source**: Full transparency, community-driven development
- **Built with Modern Tech**: Rust backend for speed, Svelte frontend for responsiveness

## 🚀 Quick Start

### Download & Install

1. **Head to [Releases](../../releases)** and download the latest installer for your platform:
   - **Windows**: `Open GFX Board_x.x.x_x64.msi`
   - **macOS**: `Open GFX Board_x.x.x_aarch64.dmg` (Apple Silicon) or Intel version
   - **Linux**: `open-gfx-board_x.x.x_amd64.deb` (Debian/Ubuntu)
2. **Run the installer** and follow the prompts
3. **Launch Open GFX Board** from your desktop
4. **Start controlling graphics!**

> ⚠️ **Note on macOS & Linux**: These platforms are community-tested and may have minor issues. Please report any bugs via [Issues](../../issues)—your feedback helps improve compatibility! Windows is the primary, fully-tested platform.

### First Time Setup

1. Create a preset (this corresponds to OBS Shortcuts)
2. Add graphics (images or video files) that you want to overlay
3. In OBS, add a Browser Source pointing to `http://localhost:PORT/obs` (use the indicated PORT in the interface, 5000 is the default)
4. Map F13-F24 hotkeys in OBS to whatever actions you want (scene switches, source toggles, etc.)
5. Use the control interface to trigger graphics in real time

## 🎯 How It Works

**Control Panel** 
 - **Left Side**: Shows all your available graphics and triggerable actions. Click any button to instantly display that graphic full-screen in your OBS overlay.
 - **Right Side**: Displays in OBS as a Browser Source and shows exactly what your audience sees. Multiple displays can be controlled simultaneously—perfect for productions with multiple camera feeds, lower thirds, or fullscreen graphics.

### Network Architecture

```
┌─────────────────────────────┐
│  Your PC (Tauri App)        │
│  - Runs the server          │
│  - Shows control interface  │
└─────────────────────────────┘
           │
           │ http://192.168.x.x:PORT
           │
    ┌──────┴──────┐
    │             │
    ▼             ▼
[Browser]  [Phone/Tablet]
[OBS Host] [Control from LAN]
```

Everyone on the same network can access the control interface. No internet required, no complicated setup.


## 🤝 Contributing

Contributions are welcome! Feel free to:
- Report bugs via Issues
- Submit feature requests
- Open Pull Requests with improvements

## 💬 Support

Have questions or running into issues? Open an [Issue](../../issues) and we'll help you out.

---

**Happy streaming!** 🎬
