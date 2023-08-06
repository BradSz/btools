# btools
Home grown development tools

## License
This is released under MIT license. I may in the future do a joint release MIT and Apache2, but I have to
research the ramifications first.

## Rationale
This is a set of development tools that I have written and/or rewritten on various projects over the years.
This repository serves as an attempt to create the best versions of those tools, compiled and unit tested.

## Installation
To use, first install `rust`/`cargo`. Afterwards, clone and install as necessary.

For convenience, a `Makefile` is provided to install all utilities simultaneously.

```bash
git clone https://github.com/BradSz/btools.git
cd btools
make install
```

# Utilities

## binspect
View a binary file, `hexdump`-style, with ability to view data as different
datatypes, including integral and floating point types.

## chop
Piping utility to fit the output of a command to fit to to a terminal window.
By default, any characters that would cause a word wrap will be truncated.
Command-line options provide the ability to customize truncating and even
wrapping behavior.
