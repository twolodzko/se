
test: unit-test integration-test

unit-test:
    cargo fmt
    cargo clippy
    cargo test

integration-test: build
    bats tests.bats

build:
    cargo build --release
    cp ./target/release/se ./

install: test
    cargo install --path .

benchmark: build
    #!/bin/bash
    set -euo pipefail

    bench() {
        if ! diff -q <(eval "$2") <(eval "$3") ; then
            echo "outputs of $2 and $3 differ"
            exit 1
        fi

        hyperfine -r "$1" --shell=none -w 5 "$2" "$3"
    }

    # small

    bench 2000 \
        'sed "/sed/ =" README.md' \
        './se -a "/sed/ =n" README.md'

    bench 2000 \
        'sed -n "/sed/ { s/default/kitty/g ; p ; }" README.md' \
        './se "/sed/ s/default/kitty/g p" README.md'

    # big

    if [ ! -f IMDB-Dataset.csv ]; then
        wget https://raw.githubusercontent.com/Ankit152/IMDB-sentiment-analysis/refs/heads/master/IMDB-Dataset.csv
    fi

    bench 100 \
        'sed -n "/love/ s/love/####/gp" IMDB-Dataset.csv' \
        './se "? s/love/####/gp" IMDB-Dataset.csv'

    bench 100 \
        'sed "/love/ s/love/####/gp" IMDB-Dataset.csv' \
        './se -a "? s/love/####/gp" IMDB-Dataset.csv'

    bench 100 \
        'sed -n "s/love/####/gp" IMDB-Dataset.csv' \
        './se "? s/love/####/gp" IMDB-Dataset.csv'

lines:
    @ find . -name '*.rs' -exec wc -l {} \;
    @ find . -name '*.rs' -exec cat {} \; | wc -l
