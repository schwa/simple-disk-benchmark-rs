# simple-disk-benchmark-rs

A simple disk benchmark tool.

![Alt text](docs/out.gif)

## Operating Systems

Currently, only macOS is tested. Linux and Windows _may_ work but are not tested.

## Installation

```sh
cargo install --git https://github.com/schwa/simple-disk-benchmark-rs
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
  -F, --use-fsync               TODO: Not implemented yet
  -m, --mode <MODE>             Types of test to run: read, write or all [default: all]
  -h, --help                    Print help
  -V, --version                 Print version
```
