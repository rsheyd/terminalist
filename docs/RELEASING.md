# Release Process

Terminalist releases are tagged from a clean `main` branch and published as GitHub releases. The
Git tag, Cargo package version, changelog heading, and GitHub release title must use the same
semantic version.

## Prepare the release

1. Synchronize `main` with the release repository and confirm the working tree is clean:

   ```bash
   git switch main
   git pull --ff-only origin main
   git status --short --branch
   ```

2. Choose a semantic version such as `0.7.2`. Replace any development suffix in `Cargo.toml` and
   update `Cargo.lock` with:

   ```bash
   cargo check
   ```

3. Convert the matching development section in `CHANGELOG.md` to a dated stable heading, such as
   `## [0.7.2] - 2026-08-03`. Summarize all user-visible changes since the previous release tag.

4. Review `README.md` for affected features, installation instructions, minimum Rust version, and
   documentation links.

## Validate and commit

Run the required checks:

```bash
cargo fmt --all -- --check
cargo test
cargo run -- --version
```

The reported version must match the intended release. Review the complete diff, then commit the
release preparation:

```bash
git diff --check
git diff
git add Cargo.toml Cargo.lock CHANGELOG.md README.md docs/README.md docs/RELEASING.md AGENTS.md
git commit -m "Prepare 0.7.2 release"
```

Replace `0.7.2` with the actual release version in this and later commands.

## Tag and publish

Create an annotated tag on the validated release commit and push both the commit and tag:

```bash
git tag -a v0.7.2 -m "Terminalist 0.7.2"
git push origin main
git push origin v0.7.2
```

Create the GitHub release using the matching changelog section as the release notes:

```bash
gh release create v0.7.2 \
  --repo rsheyd/terminalist-edge \
  --title "Terminalist 0.7.2" \
  --notes-file /path/to/release-notes.md \
  --verify-tag
```

Use a temporary release-notes file containing only the relevant changelog section, without its
heading. Do not use automatically generated notes without reviewing them against the changelog.

## Verify the published release

```bash
git ls-remote --tags origin refs/tags/v0.7.2
gh release view v0.7.2 --repo rsheyd/terminalist-edge
```

Confirm that the tag resolves to the intended commit, the release is not accidentally marked as a
draft or prerelease, and its title and notes match the changelog.

## Begin the next development cycle

When development resumes, advance `Cargo.toml` and `Cargo.lock` to the next planned development
version, for example `0.7.3-dev.1`, and add a corresponding changelog section in a separate commit.
