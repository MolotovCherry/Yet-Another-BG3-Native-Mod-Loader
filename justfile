set unstable
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]
set script-interpreter := ["pwsh.exe"]

target := `cargo target-dir`

[script]
_build profile="debug" flags="" flags2="" flags3="":
    echo "Building loader..."
    cargo build -p loader {{flags}}
    $env:LOADER_HASH = (Get-FileHash "{{target}}\{{profile}}\loader.dll" | Select-Object -ExpandProperty Hash).ToLower()
    echo "Loader hash is ${env:LOADER_HASH}"
    echo "Building yabg3nml..."
    cargo build -p yabg3nml {{flags2}}
    echo "Building autostart-installer..."
    cargo build -p autostart-installer {{flags3}}

build: _build

build-dev: (_build "debug" "" "--features test-injection" "")

build-ci: (_build "dev-ci" "--profile dev-ci" "--profile dev-ci" "--profile dev-ci")

build-release: (_build "release" "--release" "--release" "--release")
