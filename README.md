# simple-disk-benchmark

A simple disk benchmark tool.

![Alt text](docs/out.gif)

## Operating Systems

Currently, macOS and Linux are tested. Windows _may_ work but is not tested. Development is on macOS.

## Installation

```sh
cargo install simple-disk-benchmark
```

## Usage

```sh
A simple tool for benchmarking disk performance

Usage: simple-disk-benchmark [OPTIONS] [FILE]

Arguments:
  [FILE]  File to use for benchmarking. If this file exists it will be deleted [default: testfile.dat]

Options:
  -s, --size <FILESIZE>         Size of the file to use for benchmarking [default: 1GB]
  -b, --blocksize <BLOCK_SIZE>  Size of the blocks to read/write [default: 128MB]
  -c, --cycles <CYCLES>         Number of test cycles to run [default: 10]
  -m, --mode <MODE>             Types of test to run: read, write or all [default: all]
  -v, --verbose...              More output per occurrence
  -q, --quiet...                Less output per occurrence
  -h, --help                    Print help
  -V, --version                 Print version
```

## Methodology

Specify the path to a file to use for benchmarking. If the file exists, it will be deleted. You can specify paths on other drives by using the full path to the file (e.g. `/Volumes/MyDrive/testfile.dat`).

The benchmark tool will create a file of the specified size and then run the specified number of cycles. Each cycle will read or write the multiple blocks of a specified size to the file.

On macOS, the file is opened and F_NO_CACHE and F_GLOBAL_NOCACHE are both set on the file descriptor. This will bypass the file system cache and write directly to the disk. On Linux, the O_DIRECT flag is used to achieve the same result.

## TODO

* Random seeks instead of just sequential.
* Multithreaded benchmarking option.
* ~~Use random bytes instead of zeros for writes~~
* Test on Windows.
* Put on homebrew.
* More documentation.
* Run test coverage.
* Use a better ByteSize replacement.
* Borrow CPU time stuff from hyperfine.
* Better output.
* Fuzz the StyleSheet code.
* ~~Add a `--no-delete` option to keep the file around after the benchmark.~~
* Output data to CSV/Json.
* ~~Find out what's going on with all the dead_code false positives.~~
* ~~Sort out pub/mod stuff~~.

## License

MIT License. See [LICENSE](LICENSE) file.
