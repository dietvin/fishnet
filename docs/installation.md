# Installation

## Table of contents
- [Installation](#installation)
  - [Table of contents](#table-of-contents)
  - [Download and execute](#download-and-execute)
  - [Adding `fishnet` to `PATH`](#adding-fishnet-to-path)
    - [On Linux](#on-linux)
    - [On Windows](#on-windows)
  - [Uninstallation](#uninstallation)

## Download and execute

To install **fishnet**, download the appropriate binary for your system from the [releases page](https://github.com/dietvin/fishnet/releases/latest). Once downloaded, you can execute it directly from the terminal.

```bash
./path/to/fishnet-<system>-<version> --help
```

To simplify usage, consider renaming the binary to `fishnet`:

```bash
mv ./path/to/fishnet-<system>-<version> ./path/to/fishnet
./path/to/fishnet --help
```

## Adding `fishnet` to `PATH`

To use `fishnet` from any directory, add it to your system’s `PATH` variable. You can either:

1. Move the executable to a directory already in `PATH`, **or**
2. Add the directory containing the executable to your `PATH`.

### On Linux

Check your current `PATH`:

```bash
echo $PATH
```

Then either move the executable to a directory already in your `PATH`:

```bash
sudo mv ./path/to/fishnet /usr/local/bin/fishnet
```

Or create a new directory and add it to your `PATH`:

```bash
mkdir -p $HOME/bin
mv ./path/to/fishnet $HOME/bin/fishnet
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

> **Note:** If you're using `zsh`, update `~/.zshrc` instead of `~/.bashrc`.

### On Windows

1. Rename the downloaded binary (e.g., `fishnet-windows-amd64.exe`) to `fishnet.exe`.
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

---

## Uninstallation

To uninstall **fishnet**:

1. Delete the executable:
   - On Linux/macOS: 
        `rm /usr/local/bin/fishnet`
   - On Windows: 
        Delete `fishnet.exe` from its folder.

1. (Optional) Remove any custom path entry you added during installation:
   - On Linux/macOS: Edit your shell configuration file (e.g., `~/.bashrc`, `~/.zshrc`) and remove the line modifying `PATH`.
   - On Windows: Remove the fishnet directory from the system `PATH` via the Environment Variables settings.

---
