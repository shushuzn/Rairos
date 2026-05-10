$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
$oldPath = $env:PATH
$oldInclude = $env:INCLUDE
$oldLib = $env:LIB

# Run vcvarsall to set environment
& $vcvars x64 | Out-Null

# Now PATH, INCLUDE, LIB are set - run cargo
cargo build --package rairos-core --release
