#!/usr/bin/env bash
# Reduce the size of a font by removing unneeded characters from it.
# This can be done because this game won't take user input
# and therefore won't need to display arbitrary characters.

cd "$(dirname "$0")" || exit 1

if ! command -V pyftsubset; then
    echo "pyftsubset command not found." >&2
    echo "Install it with \`sudo apt install fonttools\` or \`pip install fonttools\`." >&2
    exit 1
fi

exec pyftsubset FiraSans-Regular.ttf --output-file=font.ttf \
     --unicodes=U+0020-007e
