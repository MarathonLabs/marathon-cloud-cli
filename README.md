# Marathon Cloud command-line interface

![](assets/marathon-cloud-cli.1280.gif)

## Installation
For homebrew users:
```bash
brew tap malinskiy/tap
brew install malinskiy/tap/marathon-cloud
```
To have superior experience, [enable autocompletion for Brew](https://docs.brew.sh/Shell-Completion#configuring-completions-in-zsh)

For docker users:
```bash
docker pull marathonlabs/marathon-cloud:latest
alias marathon-cloud='docker run -v "$(pwd)":/work -it --rm marathonlabs/marathon-cloud:latest'
```

## Usage
```bash
Command-line client for Marathon Cloud

Usage: marathon-cloud [OPTIONS] [COMMAND]

Commands:
  run          Submit a test run
  download     Download artifacts from a previous test run
  completions  Output shell completion code for the specified shell (bash, zsh, fish)
  help         Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help
  -V, --version     Print version
```
