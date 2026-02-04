## [1.2.2] - 2026-02-04

### 🚀 Features

- *(inject)* Added or statements to injection templating

### 📚 Documentation

- *(inject)* Add an or statment and default value to an example in the README
## [1.2.1] - 2026-02-02

### 🚀 Features

- Bitwarden sdk added, some polish with the rest, and rand cargo fmt. updated some of the readme
- Release script and hm module

### 🐛 Bug Fixes

- Config gen tool
## [1.2.0] - 2026-02-01

### 🚀 Features

- Initial work for bitwarden support
- *(env)* Add dot env added to envrc file, updates to gitignore to handle new file
- *(bws)* Add ord id to config, small updates to comments
- *(aws)* Inital setup for pulling a list and downloading the selected secrets
- Too many things to say, config local history caching and updates from ff lib. plus export and import options
- Add more config locations
- Rework the file org, moviing local actions to a jawsSecretsManager to allow future self-hosting opertunities. other stuff
- Removed editor and secrets-path flags in favor of a config flag

### 🐛 Bug Fixes

- *(nix)* Formatting the correct date, lock update
- *(dep)* Update golang dot org x net to version 0.23.0
- Config location detection
- *(tilde)* Expand tilde to place secrets in correct spot

### 💼 Other

- *(docker)* Nix docker image build added as a option
- *(nix)* Add bws to nix develop
- *(nix)* Initial try at making a home-manager module, go deps update, flake lock update
- *(nix)* Update vendorHash

### 🚜 Refactor

- Flake var name changes and other small things
- *(secretsmanager)* Moved each platform into separate folders for easier navigation
- *(config)* Moved config tmpl and funcs to cmd/jaws folder, load client refactor for all platforms

### 📚 Documentation

- Nix profile install howto
- Small formatting changes and link to anchor for a section ref
- *(install)* Added brew tap and nix install instructions
- Fixed language to reflect jaws commands
- Remove todo's
## [1.0.8] - 2024-03-26

### 🚀 Features

- *(1.0.9)* Fix build errors
## [1.0.7] - 2024-03-26

### 🚀 Features

- *(1.0.7)* Mod updates
## [1.0.6] - 2024-03-26

### ⚙️ Miscellaneous Tasks

- *(justfile)* Changed bw secret
## [1.0.5] - 2024-03-26

### 🚀 Features

- *(1.0.5)* Migrated from private project

### 📚 Documentation

- *(readme)* Remove digital ocean from goals, added build from source
## [0.1.3] - 2022-07-16

### 🐛 Bug Fixes

- *(config)* Handle config create when a config is broke, change config name

### 🚜 Refactor

- *(cmd)* New aliases for subcommands, raw version flag
- *(updates)* Thirdparty libraries updates, fuzzyfinder hotreloadlock

### 📚 Documentation

- *(readme)* Create config docs

### ⚙️ Miscellaneous Tasks

- *(token)* Using bitwarden to pull github token
## [0.1.2] - 2022-07-16

### 🚀 Features

- *(install)* Curlable install script

### 📚 Documentation

- *(readme)* Instructions for installing from install.sh

### ⚙️ Miscellaneous Tasks

- *(automation)* Figuring out version incr
- *(automation)* Using mainly goreleaser for local dev, fixed ldflags
## [0.1.1] - 2022-07-16

### 🚀 Features

- *(version)* Version command

### 🐛 Bug Fixes

- *(get)* Editior flag now opens files

### 📚 Documentation

- *(readme)* Setting git as optional dependency

### ⚙️ Miscellaneous Tasks

- *(just)* Justfile script
- *(automation)* Small changes to goreleaser and justfile
## [0.1.0] - 2022-07-16

### 🚀 Features

- *(noconfig)* Handle no config, updated readme docs
- *(config)* Inital create config command

### 🐛 Bug Fixes

- *(download)* Broke get command working now
- *(print)* Removed the secret id from the normal print

### 🚜 Refactor

- *(folders)* Moved to a more nested folder option
- *(aws)* Load client from internal aws package
- *(funcs)* Remade most functions to make more sense, no long need fzf installed
- *(rename)* Renaming project to jaws
- *(config)* Separate config command for show and create

### 📚 Documentation

- *(readme)* Formatting the title differently
- *(format)* Firm name origin

### ⚙️ Miscellaneous Tasks

- *(name)* Changing the command to firm over fc, fc is a builtin function
- *(rename)* Adding a few missed renames
- *(rename)* Updated gitignore to handle the rename
- *(rename)* More renaming, readme updates
