# WoW Addon Manager (Rust + egui)

A graphical addon manager for World of Warcraft 3.3.5 written in Rust with egui.

## Features

- Install addons from GitHub repositories
- Check for addon updates
- Update installed addons
- Graphical user interface (egui)
- YAML configuration file

## Installation

1. Make sure you have Rust installed (version 1.70 or later)
2. Clone this repository
3. Run `cargo build --release`

## Configuration

1. Copy the example configuration file:
   ```bash
   cp config.yaml.example config.yaml
   ```

2. Edit `config.yaml` and set your WoW AddOns directory path

3. Add your addon repositories to the configuration file

Example configuration:

```yaml
# WoW Addon Manager Configuration
addon_dir: "/path/to/your/wow/addons"

repositories:
  - name: "ElvUI"
    url: "https://github.com/ElvUI/ElvUI"
  - name: "WeakAuras"
    url: "https://github.com/WeakAuras/WeakAuras2"
    path: "src/WeakAuras"  # Optional: subdirectory within repo
```

### GitHub Token (Optional)

To avoid GitHub API rate limiting (60 requests/hour without token, 5,000/hour with token), you can provide a GitHub personal access token.

**Option 1: Environment Variable (Recommended - More Secure)**

1. Create a personal access token at https://github.com/settings/tokens
2. Give it `public_repo` scope (read-only access to public repositories)
3. Set the environment variable before running the application:

   Linux/macOS:
   ```bash
   export GITHUB_TOKEN="your_token_here"
   ./wow-addon-manager
   ```

   Windows (PowerShell):
   ```powershell
   $env:GITHUB_TOKEN="your_token_here"
   .\wow-addon-manager.exe
   ```

**Option 2: Configuration File (Less Secure)**

Add the token to your `config.yaml`:
```yaml
github_token: "your_token_here"
```

⚠️ **Security Warning**: If you store the token in `config.yaml`, make sure the file is in `.gitignore` and never commit it to version control!

## Usage

Run the executable to start the graphical user interface:

```
./wow-addon-manager
```

In the GUI:
- Click "Install" to install an addon
- Click "Update" to update an installed addon
- Click "Delete" to remove an installed addon
- Click "Refresh" to reload the addon list
- Click "Check Updates" to check for addon updates

## How It Works

1. The addon manager reads the configuration file to get the addon directory and list of repositories
2. For each repository, it uses the GitHub API to get the latest commit hash
3. When installing an addon, it downloads the repository as a tarball and extracts it to the addon directory
4. When updating an addon, it downloads the latest version and replaces the existing files

## Dependencies

- [egui](https://github.com/emilk/egui) for GUI framework
- [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) for application framework
- [serde](https://github.com/serde-rs/serde) for serialization
- [serde_yaml](https://github.com/dtolnay/serde-yaml) for YAML parsing
- [tokio](https://github.com/tokio-rs/tokio) for async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) for HTTP requests
- [tar](https://github.com/alexcrichton/tar-rs) for tar archive handling
- [flate2](https://github.com/rust-lang/flate2-rs) for gzip decompression
- [dirs](https://github.com/dirs-dev/dirs-rs) for platform-specific directories
- [url](https://github.com/servo/rust-url) for URL parsing

## License

This project is licensed under the MIT License.