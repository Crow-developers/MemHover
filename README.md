# MemHover

**Hover. See. Optimize.**

MemHover is a lightweight Windows background utility that displays real-time RAM usage when you hover your mouse over any application icon in the taskbar.

Built with **Rust** and **Win32 API**, it provides instant insight into which apps are consuming the most memory without opening Task Manager.

---

## ✨ Features

- **Hover to see** - Just move your mouse over any taskbar icon
- **Real-time memory usage** - Updated every 150ms
- **System tray integration** - Runs silently with exit option
- **Ultra-lightweight** - ~1-3 MB executable, no runtime dependencies
- **Native performance** - Direct Win32 API calls for minimal overhead

---

## 📋 Requirements

- **Windows 10** or **Windows 11** (uses Win32 API, won't work on Linux/macOS)
- **Rust** (only for development) - Install from [rustup.rs](https://rustup.rs)

---

## 🔧 Installation

### Option 1: Download Pre-built Binary (Recommended)

1. Go to [Releases](https://github.com/Crow-developers/MemHover/releases)
2. Download `memhover.exe`
3. Run it - no installation required

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/Crow-developers/MemHover.git
cd MemHover

# Run during development
cargo run

# Build release version
cargo build --release

# The executable will be at:
# target/release/memhover.exe
