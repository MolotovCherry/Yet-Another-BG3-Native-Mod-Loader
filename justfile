set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

target := `cargo target-dir`

_build-loader flags:
    cargo build -p loader {{flags}}

_build-yabg3nml profile flags:
    $env:LOADER_HASH = (Get-FileHash "{{target}}\{{profile}}\loader.dll" | Select-Object -ExpandProperty Hash).ToLower(); cargo build -p yabg3nml {{flags}}

build: (_build-loader "") (_build-yabg3nml "debug" "")

build-dev: (_build-loader "") (_build-yabg3nml "debug" "--features test-injection")

build-ci: (_build-loader "--profile dev-ci") (_build-yabg3nml "dev-ci" "--profile dev-ci")

build-release: (_build-loader "--release") (_build-yabg3nml "release" "--release")
