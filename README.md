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
```

## Methodology

Specify the path to a file to use for benchmarking. If the file exists, it will be deleted. You can specify paths on other drives by using the full path to the file (e.g. `/Volumes/MyDrive/testfile.dat`).

The benchmark tool will create a file of the specified size and then run the specified number of cycles. Each cycle will read or write the multiple blocks of a specified size to the file.

On macOS, the file is opened and F_NO_CACHE and F_GLOBAL_NOCACHE are both set on the file descriptor. This will bypass the file system cache and write directly to the disk. On Linux, the O_DIRECT flag is used to achieve the same result.

## TODO

* Separate file creation from opening for runs.
* Display volume info in the preamble.
* Better output -  display timing info as well as rates.
* Multithreaded benchmarking option.
* Test on Windows.
* Put on homebrew.
* More documentation.
* Run test coverage.
* Borrow CPU time stuff from [hyperfine](https://github.com/sharkdp/hyperfine).
* Fuzz the StyleSheet code.
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
