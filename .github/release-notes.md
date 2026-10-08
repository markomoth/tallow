## 🕯️ Download and play

Pick the file for your system, unpack it, and run `tallow` from a terminal that's at least **100×30**.

| System | File |
|---|---|
| 🐧 Linux | `linux-x86_64` (or `linux-x86_64-static` if that one won't start) |
| 🪟 Windows | `windows-x86_64.zip`: run `tallow.exe` in **Windows Terminal** |
| 🍎 macOS, Apple Silicon (M1 and later) | `macos-arm64` |
| 🍎 macOS, Intel | `macos-x86_64` |

**First launch warnings.** The builds aren't signed, so your system will be suspicious once:

- **macOS** says it "can't be verified". Run `xattr -d com.apple.quarantine tallow` in the unpacked folder, or right-click → Open.
- **Windows** SmartScreen may stop it. Click "More info" → "Run anyway".

Colors look wrong, or no truecolor? Try `tallow --simple`.
