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

## Autocompletions
If you're using installation from homebrew then you should have working autocompletions upon installation assuming
you've done the [brew general setup](https://docs.brew.sh/Shell-Completion#configuring-completions-in-zsh).

If you install the binary manually then you can easily generate autcompletions:

### bash
```
# set up autocomplete in bash into the current shell, bash-completion package should be installed first.
source <(marathon-cloud completions bash) 
# add autocomplete permanently to your bash shell.
echo "source <(marathon-cloud completions bash)" >> ~/.bashrc 
```

### zsh
```
# set up autocomplete in zsh into the current shell
source <(marathon-cloud completions zsh) 
# add autocomplete permanently to your zsh shell
echo '[[ $commands[marathon-cloud] ]] && source <(marathon-cloud completions zsh)' >> ~/.zshrc 
```

### fish
```
# add marathon-cloud autocompletion permanently to your fish shell 
echo 'marathon-cloud completions fish | source' >> ~/.config/fish/config.fish  
```

## License
marathon-cloud cli codebase is licensed under [MIT](LICENSE).
