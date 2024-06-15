set shell := ["fish", "-c"]

_default: _list

_list:
    just --list

update-gif:
    cargo install --path .
    vhs scripts/demo.tape -o docs/out.gif

publish:
    #!/usr/bin/env fish
    set CURRENT_BRANCH (git symbolic-ref --short HEAD)
    if [ $CURRENT_BRANCH != main ]
        echo "Not on main branch. Please switch to main before publishing."
        exit 1
    end
    set NEXT_VERSION (just _next-version)
    gum confirm "Confirm next version: '$NEXT_VERSION'?"; or exit 1
    just _check-repo; or exit 1
    cargo clippy --fix
    just _check-repo; or exit 1
    cargo test; or exit 1
    just update-usage
    echo "Updating Cargo.toml to version $NEXT_VERSION"
    toml set Cargo.toml package.version $NEXT_VERSION| sponge Cargo.toml
    gum confirm "Update gif?"; and just update-gif; git add docs/out.gif
    gum confirm "git commit -a"; and git commit -a
    gum confirm "git tag?"; and git tag $NEXT_VERSION
    gum confirm "git push?"; and git push --tags origin main
    gum confirm "Rust publish"; and cargo publish

    gum confirm "Update homebrew?"; and just homebrew-release $NEXT_VERSION

_check-repo:
    #!/usr/bin/env fish
    set is_dirty (git status --porcelain)
    if test -n "$is_dirty"
        echo "Repo is dirty. Please commit all changes before publishing."
        exit 1
    end

update-usage:
    #!/usr/bin/env fish
    awk -f scripts/replace.awk -v INDEX=2 -v "REPLACEMENT=cargo run -- --help 2> /dev/null" README.md | sponge README.md

_next-version:
    #!/usr/bin/env fish
    set LATEST_TAG (git describe --tags --abbrev=0)
    set PARTS (string split . $LATEST_TAG)
    set MAJOR $PARTS[1]
    set MINOR $PARTS[2]
    set PATCH $PARTS[3]
    set NEXT_PATCH (math $PATCH + 1)
    echo "$MAJOR.$MINOR.$NEXT_PATCH"

bonnieplusplus:
    bonnie++ | bon_csv2html > bonnie++.html

cargo-analytics:
    cargo tree > docs/tree.txt
    cargo bloat --release --crates -n 10000 > docs/bloat.txt
    cargo report future-incompatibilities > docs/future-incompatibilities.txt; or true
    unused-features analyze
    unused-features build-report --input report.json
    mv report.json docs/unused-features.json
    mv report.html docs/unused-features.html
    unused-features prune --input docs/unused-features.json

cargo-installs:
    brew install cargo-udeps
    cargo install cargo-bloat
    cargo install cargo-edit
    cargo install cargo-machete
    cargo install cargo-unused-features
    cargo install grcov
    cargo install toml-cli
    rustup component add llvm-tools-preview

linux-setup machine_name:
    orb create -a arm64 ubuntu {{machine_name}}
    orb run --machine {{machine_name}} sudo apt install -y gcc
    orb run --machine {{machine_name}} --shell curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    orb run --machine {{machine_name}} --shell ". $HOME/.cargo/env"

linux-run_ machine_name: (linux-setup machine_name)
    orb run --machine {{machine_name}} cargo clean
    orb run --machine {{machine_name}} cargo build

linux-run: (linux-run_ "ubuntu")

windows-deps:
    rustup target add x86_64-pc-windows-gnu

windows-build:
    # TODO: use cross
    cargo build --target x86_64-pc-windows-gnu

coverage:
    set -x RUSTFLAGS "-C instrument-coverage"; cargo build
    cargo test

homebrew-release VERSION:
    #!/usr/bin/env fish
    # https://federicoterzi.com/blog/how-to-publish-your-rust-project-on-homebrew/

    set VERSION {{VERSION}}
    echo $VERSION

    # Build release, create tarball and calculate sha256
    cargo build --release
    pushd target/release/
    tar -czf simple-disk-benchmark.tar.gz simple-disk-benchmark
    set SHA (shasum -a 256 simple-disk-benchmark.tar.gz | cut -d " " -f 1)
    echo $SHA
    popd

    # Create release on GitHub
    gh release create $VERSION target/release/simple-disk-benchmark.tar.gz --title "simple-disk-benchmark $VERSION"

    # Update homebrew formula
    pushd $HOME/Projects/homebrew-schwa
    git pull
    sed -i '' -e "s/sha256 \".*\"/sha256 \"$SHA\"/g" Formula/simple-disk-benchmark.rb
    sed -i '' -e "s/version \".*\"/version \"$VERSION\"/g" Formula/simple-disk-benchmark.rb
    git commit --all --message "simple-disk-benchmark $VERSION"
    git push
    popd

test-homebrew:
    brew tap schwa/schwa
    brew update
    brew install simple-disk-benchmark
