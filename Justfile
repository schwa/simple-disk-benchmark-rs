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
        echo "Not on master branch. Please switch to master before publishing."
        exit 1
    end
    set NEXT_VERSION (just _next-version)
    gum confirm "Confirm next version: '$NEXT_VERSION'?"; or exit 1
    just _check-repo; or exit 1
    cargo clippy --fix
    just _check-repo; or exit 1
    cargo test; or exit 1
    just update-usage
    gum confirm "Update gif?"; and just update-gif; git add docs/out.gif
    gum confirm "git commit -a"; and git commit -a
    gum confirm "git tag?"; and git tag -a $VERSION
    gum confirm "git push?"; and git push --tags origin main
    gum confirm "Push"; and git push --tags origin main
    gum confirm "Rust publish"; and rust publish

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

test-opens:
    cargo build --release
    rm -f test-1.log
    rm -f test-2.log
    ./target/release/simple-disk-benchmark --size 1GB --blocksize 128MB --cycles 10 --mode read --no-progress --no-chart --export-log test-1.log
    ./target/release/simple-disk-benchmark --size 1GB --blocksize 128MB --cycles 10 --mode read --no-progress --no-chart --export-log test-2.log --no-close-file
    grep "posix::open" test-1.log | wc -l
    grep "posix::open" test-2.log | wc -l

bonnieplusplus:
    bonnie++ | bon_csv2html > bonnie++.html
