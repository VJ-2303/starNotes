# Releasing Castle

Castle releases are built by GitHub Actions for Linux x86-64. Each release contains a standalone `castle` binary packaged as a `.tar.gz` archive and a `SHA256SUMS.txt` checksum file.

## Publish a release

1. Update `version` under `[workspace.package]` in `Cargo.toml`.
2. Refresh the lockfile and verify the release build locally:

   ```sh
   cargo check --workspace --all-targets
   cargo build --locked --release --package app --bin castle
   ```

3. Commit the version change:

   ```sh
   git add Cargo.toml Cargo.lock
   git commit -m "release: vX.Y.Z"
   ```

4. Tag that commit and push the commit and tag:

   ```sh
   git tag vX.Y.Z
   git push origin HEAD
   git push origin vX.Y.Z
   ```

The `Release` workflow checks that the tag matches the Cargo version, installs system dependencies, builds the optimised binary, packages it as a `.tar.gz`, and creates the GitHub release with generated release notes.

To rerun publishing for an existing tag, open **Actions → Release → Run workflow** and enter the tag. Existing assets with the same names are replaced.

## Version requirements

Release tags must use `vMAJOR.MINOR.PATCH`, for example `v0.4.0`. The tag without its leading `v` must exactly match the workspace version in `Cargo.toml`.
