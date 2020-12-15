# pj

Simple unix command line tool to find occurrences of sentinel files in a
directory structure. Kind of like your favorite `grep` but it terminates a
search if it finds a sentinel.

## Usage

```bash
$ pj # assumes you're looking for .git files
# src/projectname1 src/projectname2 src/projectname3
# ...

$ pj .hg # for if you work at Facebook
no projects found
```

Note that this is actaully all fake I haven't made it yet.

## Story Time

Imagine you're running <INSERT FAVORITE EDITOR> and you have the age old
question: 

> How do I switch to this other project I'm working on?

You say to yourself, wow it would be nice if I had something like
[projectile](https://github.com/bbatsov/projectile) now that I'm using Vim. But
you're not on LISP machine's wild ride any more--you don't get to have a nice
monoloth. You make your own little
processes and you put them together until you've gone and made yourself ~a
robot~ ~a multimillion dollar company~ a cute little house of cards!

So you download [denite](https://github.com/Shougo/denite.nvim) because it's
pretty cool and then you get text finding (with `ag`) and file finding (also
with `ag`) and then you're like:

> Wish I could just use `ag` to find my Git repos but I don't want to index all
> that extra stuff

So you *literally* just make [ripgrep](https://github.com/BurntSushi/ripgrep)
except worse and it only finds sentinel files.

## License  
