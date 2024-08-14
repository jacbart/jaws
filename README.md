# JAWS  

>work in progress

This project was inspired by AWS not having the best UX for their secrets management. This tool uses a fuzzy finder to make filtering and selecting of multiple secrets easy. Once you have the secrets downloaded just edit the files as you would like and run the set command to update the secrets.

# Guides

[Getting Started](./docs/getting-started.md)  
[How to Install](./docs/install.md)  
[Configuring Jaws](./docs/configure.md)  
[Managing Project Environment Files](./docs/manage-env.md)  

# Demo

![demo](./docs/vhs/demo/pull_edit_push.gif)

# Commands
## Secrets Manager
- pull
- push
- list
- add
- delete
- rollback
## Self
- config - displays basic info on the current config
	- create - create a new config
	- show - display the config contents
	- path - show the current config path
	- edit - open the current config using the `$EDITOR` env variable
	- lock - Encrypt the current config with a password or using `$JAWS_CONFIG_KEY` env variable
	- unlock - Decrypt the current config with a password or using `$JAWS_CONFIG_KEY` env variable
- clean - clean local secrets by deleting the path
- completion - shell completions
- diff - git diff for downloaded secrets
- path - display the current secrets download path
	- command - prints a shell function to `popd` and `pushd` to and from the secrets path
- status - git status for the downloaded secrets
- update - self update command
- version - display jaws version

# Platforms

## AWS

- [x] pull
	- [x] suggest secret if you miss-type the ID
- [x] push
- [x] list
- [x] add
- [x] delete
- [ ] rollback - partial working, no rollback choice

## GCP

- [x] pull - partial working
	- [x] fuzzy pull a secret using the args, if `testing_key` is passed look for `projects/projectID/secrets/testing_key`
- [x] push
- [x] list
- [x] add
- [x] delete
- [x] rollback

## Bitwarden Secrets

- [ ] pull
- [ ] push
- [ ] list
- [ ] add
- [ ] delete
- [ ] rollback

# Environment File Manager

**purpose**: Using a config file, output a var file that can be consumed at runtime. Using an integration with aws or gcp's secret manager pull secrets and use them as values for keys set in `whatever.jaws`. Using this instead of a local `.env` can prevent secrets from being leaked or accidentally committed to a repo, it also lets a developer have multiple environments declared in the config i.e. dev, testing, or production.

## Input

- config file in hcl format
	- vars
		- secrets - `secret`
		- local and env variables - `var`
			- an environment variable will override one set in the locals block
	- functions
		- quote
		- encode
		- decode
		- file
		- sh
		- resolve
		- escape
		- input
	- [operators](https://developer.hashicorp.com/terraform/language/expressions/operators)
	- [conditionals](https://developer.hashicorp.com/terraform/language/expressions/conditionals)

## Output

>output can print to stdout or to a file directly.

- shell variable file i.g. `.env` 
- json 
- yaml 
- tfvars 

