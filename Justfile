update-gif:
    vhs docs/demo.tape -o docs/out.gif

publish VERSION:
    cargo test
    just update-usage
    just update-gif
    @echo git add docs/out.gif
    @echo git commit -m "Update demo gif"
    @echo git push
    @echo git tag -a {{VERSION}}
    @echo git push --tags
    @echo rust publish

update-usage:
    #!/usr/bin/env fish
    awk -f scripts/replace.awk -v INDEX=2 -v "REPLACEMENT=cargo run -- --help 2> /dev/null" README.md | sponge README.md
