# pj

A simple command-line tool to find directories which contain a specific file.

## Usage

It comes with a nice `--help` command :)

```shell
$ pj --help
pj 0.1.2
A fast sentinel file finder.

USAGE:
    pj [OPTIONS] <sentinel-pattern> [root-dirs]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --depth <depth>

ARGS:
    <sentinel-pattern>
    <root-dirs>...
```

For example, find all of your git repositories under your ~/src directory:

```shell
pj '\.git' ~/src
```

## License

MIT Open Source, refer to `LICENSE` file for details.
