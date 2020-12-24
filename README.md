# pj

<img src="/res/logo.svg" alt="pj logo">

Simple unix command line tool to find occurrences of sentinel files in a
directory structure. Kind of like your favorite `grep` but it terminates a
search if it finds a sentinel.

## Why?

I recently moved from [Emacs](https://www.gnu.org/software/emacs/) to
[Neovim](https://neovim.io/) because I had configured into a hole (see: Emacs
was freezing and I couldn't figure out why). I really got into the long-living
editor process, and I wanted to recreate the killer feature that drove it:
[projectile](https://github.com/bbatsov/projectile).

I found [denite.nvim](https://github.com/Shougo/denite.nvim) which provided me
file and text search out of the box (more or less), but it couldn't swap between
projects! I was doomed!

And then I thought "hey I've been intending to learn Rust for a while, what if I
made a high-perf project finder." Then I did!

Traversing ~16000 directories:

```
pj .git ~/src  0.28s user 2.15s system 1409% cpu 0.173 total
```

Traversing a much more reasonable number (that I haven't counted):

```
pj .git ~/src --depth=2  0.01s user 0.03s system 457% cpu 0.008 total
```

After I was done with `pj` I just hooked it up into Denite like
[so](https://github.com/crockeo/nvim/blob/6e19018c9a4d015aaed3dab40b8ce7efee59a60f/rplugin/python3/denite/source/pj.py)
and then I had my projectile back ❤️.

## Usage

```bash
$ pj .git ~/src  # searches ~/src for all directories that contain .git, e.g.
~/src/cool_project_1
~/src/kind_of_cool_project
~/src/hip_name
~/src/sub_dir/buried_project
...
```

If you know your projects have a relatively flat structure, you can also use
the `--depth` command to limit the depth of the search:

```bash
$ pj .git ~/src --depth=1
~/src/cool_project_1
~/src/kind_of_cool_project
~/src/hip_name
# does *not* find
# ~/src/sub_dir/buried_project
```

## License

MIT Open Source, refer to `LICENSE` file for details.
