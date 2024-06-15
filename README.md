# simple-disk-benchmark

A simple disk benchmark tool.

![Alt text](docs/out.gif)

## Operating Systems

Currently, macOS and Linux are tested. Windows support is limited, unit tests pass, but the software is not otherwise tested or validated on Windows. Development is on macOS.

## Installation

From homebrew (macOS only):

```sh
brew tap schwa/schwa
brew install simple-disk-benchmark
```

Or with cargo (all platforms):

```sh
cargo install simple-disk-benchmark
```

## Usage

```sh
A simple disk benchmark tool

Usage: simple-disk-benchmark [OPTIONS] [FILE]

Arguments:
  [FILE]  File to use for benchmarking. If this file exists it will be deleted [default: testfile.dat]

Options:
  -s, --size <FILESIZE>         Size of the file to use for benchmarking [default: 1GB]
  -b, --blocksize <BLOCK_SIZE>  Size of the blocks to read/write [default: 128MB]
  -c, --cycles <CYCLES>         Number of test cycles to run [default: 10]
  -m, --mode <MODE>             Types of test to run: read, write or all [default: all] [possible values: all, read, write]
  -r, --random-seek             Seek to a random position in the file before each read/write
      --no-create               Do not create the test file, the file must already exist
      --no-delete               Do not delete the test file after the test
      --no-progress             Do not display progress bar
      --no-disable-cache        Do not disable the file system cache
      --no-close-file           Do not close the file after each cycle
      --no-random-buffer        Fill the buffer with fixed byte pattern on creation instead of random
  -X, --no-chart                Do not display a bar chart of the run timings
  -j, --export-json <FILE>      Export the timing summary statistics and timings of individual runs as JSON to the given FILE. The output time unit is always seconds
      --export-log <FILE>       Export the log to the given FILE
  -d, --dry-run                 Do not actually perform benchmarks to the disk (file is still created and/or deleted)
  -v, --verbose...              Increase logging verbosity
  -q, --quiet...                Decrease logging verbosity
  -h, --help                    Print help
  -V, --version                 Print version
```

## Methodology

Specify the path to a file to use for benchmarking. If the file exists, it will be deleted. You can specify paths on other drives by using the full path to the file (e.g. `/Volumes/MyDrive/testfile.dat`).

The benchmark tool will create a file of the specified size and then run the specified number of cycles. Each cycle will read or write the multiple blocks of a specified size to the file.

On macOS, the file is opened and F_NO_CACHE and F_GLOBAL_NOCACHE are both set on the file descriptor. This will bypass the file system cache and write directly to the disk. On Linux, the O_DIRECT flag is used to achieve the same result.

## TODO

* Display volume info in the preamble.
* Better output -  display timing info as well as rates.
* Multithreaded benchmarking option.
* More documentation.
* Run test coverage.
* Borrow CPU time stuff from [hyperfine](https://github.com/sharkdp/hyperfine).
* Fuzz the StyleSheet code.
* ~~Put on homebrew~~.
* ~~Separate file creation from opening for runs.~~
* ~~Test on Windows.~~
* ~~Random seeks instead of just sequential.~~
* ~~Use random bytes instead of zeros for writes~~
* ~~Use a better ByteSize replacement.~~
* ~~Add a `--no-delete` option to keep the file around after the benchmark.~~
* ~~Output data to JSON~~.
* ~~Output data to CSV~~. Won't do this. Use JSON and pipe to `jq` or `csvkit`.
* ~~Find out what's going on with all the dead_code false positives.~~
* ~~Sort out pub/mod stuff~~.

## License

MIT License. See [LICENSE](LICENSE) file.
