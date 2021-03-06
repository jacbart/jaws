# Just A Working Secretsmanager or JAWS

This project was insired by AWS not having the best UX for their secrets management. This tool uses a fuzzy finder to make filtering and selecting of multiple secrets easy. Once you have the secrets downloaded just edit the files as you would like and run the set command to update the secrets.

Rollback currently only lets there be 2 total versions of each secret so you can only rollback once. **DOUBLE CHECK YOUR WORK BEFORE UPLOADING**

For info on how to use this tool the `--help/-h` option will work on the root `jaws -h` command as well as all sub commands i.e. `jaws get -h`.

## Dependencies

- git (optional for `jaws diff` command)

## Install latest released binary

This script will download the latest released jaws binary for your system and move it into `~/.local/bin`.

```bash
curl -sfL https://raw.githubusercontent.com/jacbart/jaws/main/install.sh | bash
```

## Install jaws with golang

**Dependencies**  

- golang >=1.18

```bash
go install github.com/jacbart/jaws/cmd/jaws@latest
```

## Build from source

```bash
CGO_ENABLED=0 go build -ldflags "-s -w -X 'main.Version=0.1.4-rc' -X 'main.Date=today'" ./cmd/jaws
```

## Configure jaws

jaws will look for a config in these folders in order  
1. **./jaws.conf**  
2. **~/.jaws/jaws.conf**  
3. **~/.config/jaws/jaws.conf**  

Secret Manager Compatibility:
| Platform              | Working? |
| --------------------- | -------- |
| Amazon Web Services   | Yes      |
| Google Cloud Platform | No       |
| Hasicorp Vault        | No       |

Generate new config
```sh
jaws config create > jaws.conf
```

```
general {
  default_profile = "default"
  editor = ""
  secrets_path = ""
}

manager "aws" "default" {
  access_id = ""
  secret_key = ""
  region = ""
} # if no creds are provided jaws will use the ~/.aws/credentials or standard environment variables
```

The `secrets_path` can be set with the `--path` flag and the `editor` can be set with the `$EDITOR` environment variable.

## jaws Examples

```bash
# pulls a list of secrets into a fuzzy finder, select secrets with tab and press enter
# to confirm selection
jaws get

# create the folder stucture and an empty file then open with editor
jaws create -e testing/fake/example/secret

# add cd command to shell
jaws path command >> ~/.bashrc
# then source or restart your terminal jcd should then work
# or
# load the command into your current session only
source <(jaws path command)
# jawsd or jaws-cd toggles between your current directory and the secrets folder in your jaws.conf file
jd
# or
jaws-cd

# pushes all secrets in the secrets folder, and prompts user if there
# are any new secrets found (Deletes all local secrets as well --keep
# if you want to keep them locally)
jaws set

# pulls a list of secrets into a fuzzy finder, select the secrets you want to rollback a
# version with tab and hit enter to confirm selection
jaws rollback

# to schedule secret(s) for deletion
jaws delete --days 30

# to cancel the deletion you need to specify the secret name
jaws delete cancel testing/fake/example/secret

# remove local secrets (basically rm -rf /path/to/secrets)
jaws clean
```
