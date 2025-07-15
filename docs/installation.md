# Installation

## Table of contents
- [Installation](#installation)
  - [Table of contents](#table-of-contents)
  - [Download and execute](#download-and-execute)
    - [For Linux users:](#for-linux-users)
    - [For Windows users:](#for-windows-users)
  - [Adding `fishnet` to `PATH`](#adding-fishnet-to-path)
    - [On Linux](#on-linux)
    - [On Windows](#on-windows)
  - [Uninstallation](#uninstallation)


## Download and execute

To install **fishnet**, download the appropriate binary for your system from the [releases page](https://github.com/dietvin/fishnet/releases/latest) or directly from below:

- **Linux x86_64**: [fishnet-linux-x86_64.tar.gz](https://github.com/dietvin/fishnet/releases/latest/download/fishnet-linux-x86_64.tar.gz)
- **Linux ARM64**: [fishnet-linux-aarch64.tar.gz](https://github.com/dietvin/fishnet/releases/latest/download/fishnet-linux-x86_64.tar.gz)
- **Windows**: [fishnet.exe](https://github.com/dietvin/fishnet/releases/latest/download/fishnet.exe)

### For Linux users:

Extract the downloaded archive:
```bash
tar -xzf fishnet-linux-x86_64.tar.gz
# or
tar -xzf fishnet-linux-aarch64.tar.gz
```

This creates a single binary `fishnet`. To execute the binary:
```bash
./fishnet --help
```

### For Windows users:

Simply download `fishnet.exe` and run it directly:
```cmd
fishnet.exe --help
```


## Adding `fishnet` to `PATH`

To use `fishnet` from any directory, add it to your system's `PATH` variable. You can either:
1. Move the executable to a directory already in `PATH`, **or**
2. Add the directory containing the executable to your `PATH`.

### On Linux

Check your current `PATH`:
```bash
echo $PATH
```

Then either move the executable to a directory already in your `PATH`:
```bash
mv ./fishnet /usr/local/bin/fishnet
```

Or create a new directory and add it to your `PATH`:
```bash
mkdir -p $HOME/bin
mv ./fishnet $HOME/bin/fishnet
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

> **Note:** If you're using `zsh`, update `~/.zshrc` instead of `~/.bashrc`.

### On Windows

1. Download `fishnet.exe` from the releases page.
2. Move `fishnet.exe` to a permanent location (e.g., `C:\Tools\fishnet\fishnet.exe`).
3. Add the directory to the system `PATH`:
   - Press `Win + R`, type `sysdm.cpl`, and hit Enter.
   - Go to the **Advanced** tab and click **Environment Variables**.
   - Under **System variables**, find and select `Path`, then click **Edit**.
   - Click **New** and add the path to the folder containing `fishnet.exe` (e.g., `C:\Tools\fishnet`).
   - Click **OK** to save and exit all dialogs.
4. Open a new Command Prompt and run:
```cmd
fishnet --help
```


## Uninstallation

To uninstall **fishnet**:

1. Delete the executable:
   - On Linux/macOS:
     ```bash
     rm /usr/local/bin/fishnet
     ```
   - On Windows:
     Delete `fishnet.exe` from its folder.

2. (Optional) Remove any custom path entry you added during installation:
   - On Linux/macOS: Edit your shell configuration file (e.g., `~/.bashrc`, `~/.zshrc`) and remove the line modifying `PATH`.
   - On Windows: Remove the fishnet directory from the system `PATH` via the Environment Variables settings.

---