set shell := ["powershell.exe", "-c"]

build:
    cargo build -p loader
    cargo build -p yabg3nml

build-dev:
    cargo build -p loader
    cargo build -p yabg3nml --features test-injection

build-release:
    cargo build -p loader --release
    cargo build -p yabg3nml --release
