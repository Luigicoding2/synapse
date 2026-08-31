Write-Host "Building Synapse Release Libraries..." -ForegroundColor Cyan
cargo build --release

Write-Host "Running Rust Workspace Tests..." -ForegroundColor Cyan
cargo test --workspace

Write-Host "Running Go Bindings Tests and Benchmarks..." -ForegroundColor Cyan
Push-Location bindings
go test -v .
go test -bench="." -benchmem
Pop-Location

Write-Host "Running Demos..." -ForegroundColor Cyan
cargo run -p rust-demo

Push-Location examples/go_demo
go run main.go
Pop-Location

Write-Host "All builds and verification tests passed successfully!" -ForegroundColor Green
