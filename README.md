# pj

A simple command-line tool to find directories which contain a specific file.

## Usage

It comes with a nice `--help` command :)

```shell
$ pj --help
pj 0.2.0
A fast sentinel file finder.

USAGE:
    pj [OPTIONS] <sentinel-pattern> [--] [root-dirs]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --depth <depth>
        --ignore <ignore>...

ARGS:
    <sentinel-pattern>
    <root-dirs>...```

For example, find all of your git repositories under your ~/src directory
while ignoring directories `go`, `venv` and `node_modules`:

```shell
pj '\.git' --ignore go venv node_modules -- ~/src
```

## License

MIT Open Source, refer to `LICENSE` file for details.
