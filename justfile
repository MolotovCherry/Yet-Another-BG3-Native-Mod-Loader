set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

target := `cargo target-dir`

_build profile flags flags2:
    cargo build -p loader {{flags}}
    $env:LOADER_HASH = (Get-FileHash "{{target}}\{{profile}}\loader.dll" | Select-Object -ExpandProperty Hash).ToLower(); cargo build -p yabg3nml {{flags2}}

build: (_build "debug" "" "")

build-dev: (_build "debug" "" "--features test-injection")

build-ci: (_build "dev-ci" "--profile dev-ci" "--profile dev-ci")

build-release: (_build "release" "--release" "--release")
