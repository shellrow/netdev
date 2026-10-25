# target platforms
$targets = @(
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-freebsd",
    "aarch64-linux-android",
    "x86_64-linux-android"
)

# Build scripts and proc macros run inside the container. Keep their artifacts
# separate because cross images can have incompatible host glibc versions.
$originalTargetDir = $env:CARGO_TARGET_DIR
$targetDir = if ([string]::IsNullOrEmpty($originalTargetDir)) { "target" } else { $originalTargetDir }
$buildRoot = Join-Path $targetDir "build"

try {
    foreach ($target in $targets) {
        $env:CARGO_TARGET_DIR = Join-Path $buildRoot $target
        Write-Host "==> Building for $target..."
        & cross build --target $target
        if ($LASTEXITCODE -ne 0) {
            Write-Error "❌ Build failed for $target"
            exit 1
        }
        Write-Host "✅ Success: $target"
    }
    Write-Host "✅ All builds succeeded."
} finally {
    # Restore the caller's environment even when a build fails.
    $env:CARGO_TARGET_DIR = $originalTargetDir
}
