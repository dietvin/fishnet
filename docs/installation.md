# Installation

=== "Linux"
    Download the archive:

    * [fishnet-linux-x86_64.tar.gz](https://github.com/dietvin/fishnet/releases/latest/download/fishnet-linux-x86_64.tar.gz) (x86-64)
    * [fishnet-linux-aarch64.tar.gz](https://github.com/dietvin/fishnet/releases/latest/download/fishnet-linux-x86_64.tar.gz) (arm64)

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

    To call fishnet from any directory, place the binary in a directory that is part of the `$PATH`:
    ```bash
    mv fishnet /usr/local/bin/fishnet
    ```

    Alternatively, add a new directory to `$PATH`:
    ```bash
    mkdir -p $HOME/bin
    mv ./fishnet $HOME/bin/fishnet
    echo 'export PATH="$HOME/bin:$PATH"' >> ~/.bashrc
    source ~/.bashrc
    ```
    > **Note:** If you're using `zsh`, update `~/.zshrc` instead of `~/.bashrc`.


=== "Windows"
    Download [fishnet.exe](https://github.com/dietvin/fishnet/releases/latest/download/fishnet.exe) and run it in the command line:
    ```cmd
    fishnet.exe --help
    ```

    To call fishnet from any directory, place the binary in a directory that is part of the `PATH`:

    1. Move `fishnet.exe` to a permanent location (e.g., `C:\Tools\fishnet\fishnet.exe`).
    2. Add the directory to the system `PATH`:
       - Press `Win + R`, type `sysdm.cpl`, and hit Enter.
       - Go to the **Advanced** tab and click **Environment Variables**.
       - Under **System variables**, find and select `Path`, then click **Edit**.
       - Click **New** and add the path to the folder containing `fishnet.exe` (e.g., `C:\Tools\fishnet`).
       - Click **OK** to save and exit all dialogs.
