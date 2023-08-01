update-gif:
    vhs docs/demo.tape -o docs/out.gif

publish VERSION:
    echo just update-gif
    echo git add docs/out.gif
    echo git commit -m "Update demo gif"
    echo git push
    echo git tag -a {{VERSION}}
    echo git push --tags
    echo rust publish
