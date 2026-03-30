{
  description = "AI Agent OS - Native system monitoring and log analysis agent";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        # Build dependencies
        buildInputs = with pkgs; [
          systemd
          libxkbcommon
          wayland
          openssl
          libsoup_3
          # Tauri dependencies
          webkitgtk_6_0
          gtk3
          cairo
          gdk-pixbuf
          glib
          dbus
          librsvg
          libgnurl
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
        ];

        # Rust package
        ai-agent = pkgs.rustPlatform.buildRustPackage {
          pname = "ai-agent";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit buildInputs nativeBuildInputs;

          meta = with pkgs.lib; {
            description = "AI Agent OS - Native system monitoring and log analysis";
            homepage = "https://github.com/VoidNxSEC/ai-agent-os";
            license = licenses.mit;
            maintainers = [ marcosfpina ];
          };
        };

      in
      {
        packages = {
          default = ai-agent;
          ai-agent = ai-agent;
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
            cargo-watch
            cargo-edit
            cargo-audit
            clippy
            rustfmt
          ]);

          shellHook = ''
            echo "🤖 AI Agent OS Development Shell"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo run            - Run the agent"
            echo "  cargo test           - Run tests"
            echo "  cargo clippy         - Run linter"
            echo "  cargo watch -x run   - Auto-rebuild and run"
            echo ""
            echo "Rust version: $(rustc --version)"
          '';
        };

        # Formatter
        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
