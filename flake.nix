{
  description = "An embedded Rust development environment for the Arduino Nano ESP32";

  # inputs are the dependencies of the flake
  inputs = {
    # Specify where nixpkgs come from
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-24.11";
  };

  # outputs are the actual things that the flake produces
  outputs = { self, nixpkgs }: 
    let
      # List of supported architectures
      supportedSystems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];

      # Function to produce a development shell for any given system
      mkDevShell = system:
        let 
          pkgs = import nixpkgs {system = system;};

          rustVersion = "1.84.0";
        in pkgs.mkShell {
          # Establish the required binaries and libraries for embedded Rust development
          buildInputs = [
            pkgs.rustup # Rust version manager
            pkgs.espup # ESP development bootstrapper
          ];

          # Setup the shell environment for embedded Rust development;
          # Install the Rust toolchain, the embedded target, and set the linker 
          shellHook = ''
            echo "👷 Welcome to the Embedded Rust DevShell!";

            # Ensure Rust toolchain is installed
            if ! rustup show 2>/dev/null; then
              echo "🦀 Installing Rust toolchain...";
              rustup install ${rustVersion}
              rustup default ${rustVersion}
            fi

            # Set up Rust language server
            if ! rustup component list --installed | grep -q "rust-analyzer"; then
              echo "🔧 Installing rust-analyzer...";
              rustup component add rust-analyzer
            fi

            # Install ESP development prerequesites
            espup install

            # Set alias for executing export-esp.sh
            echo "running export-esp.sh..."
            . $HOME/export-esp.sh

            # Setting alias for export-esp.sh
            echo "Setting alias for export-esp.sh..."
            alias get_esprs='. $HOME/export-esp.sh'
            
            # Install esp-generate templating
            echo "installing esp-generate..."
            cargo install esp-generate
            echo "Done."

            # Install espflash
            echo "installing espflash..."
            cargo install espflash --locked
            echo "Done."

            # fin
            echo "🐚 Welcome to Rust ESP Development! 🦀"
          '';
        };

      # helper function to get nixpkgs for a specific platform
      # maps over provided systems, returning a list of attributes mapping the name to its packages value
      forAllSystems = systems: f: builtins.listToAttrs (map (system: {
        name = system;
        value = f system;
      }) systems);

    in {
      # Define the development shell for each supported system
      devShell = forAllSystems supportedSystems mkDevShell;
  };
}
