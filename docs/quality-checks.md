# Quality checks

The general CI workflow runs cargo xtask ci on every pull request and push to main on
Linux and Windows, using Rust 1.95.0. The existing gate checks formatting, release-profile
Clippy and workspace tests, dependency layering, assets and WebAssembly compilation.
Both platforms must find ffmpeg and ffprobe before running tests. The aggregate check
is **quality gate**; select it in branch rules after the workflow has run.

AV1 round-trip oracles require FFmpeg's `libdav1d` decoder. Windows installs
`ffmpeg-full`; CI verifies the decoder before testing. The
[build publisher](https://www.gyan.dev/ffmpeg/builds/) lists the available packages.

## Windows Intel Vulkan reference effects

The existing CPU/GPU comparisons reproduce output differences on Windows Intel Arc Vulkan
for Curl Noise, CC Drizzle, Posterize, Photoshop Arbitrary Map, CC Grid Wipe and
VR Chromatic Aberrations. Windows Intel Vulkan devices use the CPU reference implementation
for these six effects inside the GPU compositor, preserving buffer geometry and
quantisation. This costs readbacks and CPU time; full GPU acceleration for these
effects still needs a precision fix. The comparison tolerances remain unchanged.

```sh
cargo test -p effectcraft-gpu --lib --release
```

## Loaded project invariants

Project::from_json and Session::replace_project reject composition cycles, missing
composition references and invalid ID ranges before the project reaches the renderer.
Graph validation and comp_contains use iterative traversal with visited states.

replace_project returns a Result. Propagate or handle its error before treating a load as
successful; validation happens before cancelling work in the current session.

Counter repair and builder allocation cannot wrap u64. The engine checks allocation
headroom before and after an edit, so an exhausted candidate cannot publish state or an
undo entry. Public model builders remain infallible; validate externally constructed
projects at the loading boundary.

```sh
cargo test -p effectcraft-project --lib
cargo test -p effectcraft-engine --lib tests_project_validation
cargo test -p effectcraft-engine --lib worker_migration_exhaustion
```

The worker regression verifies that migration can reject an exhausted legacy project,
report failure and finish cleanup while preserving the current session.
