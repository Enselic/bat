# Release checklist

## Version bump

- [ ] Update version in `Cargo.toml`. Run `cargo build` to update `Cargo.lock`.
      Make sure to `git add` the `Cargo.lock` changes as well.
- [ ] Find the current min. supported Rust version by running
      `cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].rust_version'`.
- [ ] Update the version and the min. supported Rust version in `README.md` and
      `doc/README-*.md`. Check with
      `git grep -i -e 'rust.*1\.' -e '1\..*rust' | grep README | grep -v tests/`.

## CHANGELOG.md updates

- [ ] Compare current `CHANGELOG.md` against
  `tag=$(git tag --sort=-creatordate | grep ^v | head -n1) ; git diff $tag` and add
  missing entries. Expect in particular dependabot PRs to not be in
  `CHANGELOG.md` since they are [auto-merged] if CI passes.
- [ ] Introduce a section for the new release and perform final touch-ups.
  The GitHub Release notes will be taken from `CHANGELOG.md` with the
  `parse-changelog` crate.

## Update syntaxes and themes (build assets)

- [ ] Install the latest master version (`cargo clean && cargo install --locked -f --path .`) and make
      sure that it is available on the `PATH` (`bat --version` should show the
      new version).
- [ ] Run `assets/create.sh` and check in the binary asset files.

## Documentation

- [ ] Review [`-h`](./short-help.txt), [`--help`](./long-help.txt), and the `man` page. The `man` page is shown in
      the output of the CI job called *Documentation*, so look there.
      The CI workflow corresponding to the tip of the master branch is a good place to look.

## Pre-release checks

- [ ] Push all changes and wait for CI to succeed (before continuing with the
      next section).
- [ ] Optional: manually test the new features and command-line options. To do
      this, install the latest `bat` version again (to include the new syntaxes
      and themes).
- [ ] Run `cargo publish --dry-run` to make sure that it will
      succeed later.

## Release

1. Run https://github.com/sharkdp/bat/actions/workflows/Release.yml workflow from `main` ([instructions](#how-to-trigger-main-branch-workflow))
1. Done!

TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO 
HOW TO FIX? Parameterize IS_RELEASE on CICD.yml and set from calling workflow?
- [ ] Check if the binary deployment works (archives and Debian packages should
      appear when the CI run for the Git tag has finished).
TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO TODO 

### How to trigger main branch workflow

1. Go to https://github.com/cargo-public-api/cargo-public-api/actions and select workflow in the left column
1. Click the **Run workflow ▼** button to the right
1. Make sure the `main` branch is selected
1. Click **Run workflow**
1. Wait for the workflow to complete

## Post-release

- [ ] Prepare a new "unreleased" section at the top of `CHANGELOG.md`.
      Put this at the top:

```
# unreleased

## Features

## Bugfixes

## Other

## Syntaxes

## Themes

## `bat` as a library


```

[auto-merged]: https://github.com/sharkdp/bat/blob/master/.github/workflows/Auto-merge-dependabot-PRs.yml
