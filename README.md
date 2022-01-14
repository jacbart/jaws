# Fidelius Charm or Firm
>**FI**delius cha**RM** -> FIRM

AWS does not have the best UX for their secrets management. This tool uses a fuzzy finder to make filtering and selecting of multiple secrets easy. Once you have the secrets downloaded just edit the files as you would like and run the set command to update the secrets.

Rollback currently only lets there be 2 total versions of each secret so you can only rollback once. **DOUBLE CHECK YOUR WORK BEFORE UPLOADING**

For info on how to use this tool the `--help/-h` option will work on the root `firm -h` command as well as all sub commands i.e. `firm get -h`.

## Dependencies

- golang >=1.17

## Install/Update firm with golang

```bash
go install github.com/jacbart/fidelius-charm/cmd/firm@latest
```

## Configure firm

**~/.config/firm/firm.config** or **~/firm.config** or **./firm.config**
```
general {
  default_profile = "default"
  editor = ""
  secrets_path = ""
}

manager "aws" "default" {} # if no creds are provided firm will use the ~/.aws/credentials
```

The `secrets_path` can be set with the `--path` flag and the `editor` can be set with the `$EDITOR` environment variable.

## firm Examples

```bash
# pulls a list of secrets into fzf, select secrets with tab and press enter
# to confirm selection
firm get

# create the folder stucture and an empty file then open with editor
firm create -e testing/fake/example/secret

# add cd command to shell
firm path command >> ~/.bashrc
# then source or restart your terminal jcd should then work
# or
# load the command into your current session only
source <(firm path command)
# firmd or firm-cd toggles between your current directory and the secrets folder in your firm.config file
firmd
# or
firm-cd

# pushes all secrets in the secrets folder, and prompts user if there
# are any new secrets found (Deletes all local secrets as well --keep
# if you want to keep them locally)
firm set

# pulls a list of secrets into fzf, select the secrets you want to rollback a
# version with tab and hit enter to confirm selection
firm rollback

# to schedule secret(s) for deletion
firm delete --days 30

# to cancel the deletion you need to specify the secret name
firm delete cancel testing/fake/example/secret

# remove local secrets (basically rm -rf /path/to/secrets)
firm clean
```
