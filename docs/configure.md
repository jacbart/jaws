[Back to Readme](../README.md)

# Setup

## Jaws configuraion

jaws will look for a config in these folders in order

1. **./jaws.conf**
2. **~/.jaws/jaws.conf**
3. **~/.config/jaws/jaws.conf**

Secret Manager Compatibility:
| Platform | Working? |
| --------------------- | -------- |
| Amazon Web Services | Yes |
| Google Cloud Platform | Yes |

Generate blank config

```sh
jaws config create > jaws.conf
```

```
general {
  default_profile = "default"
  safe_mode = true
  repo_warn = false
  secrets_path = "secrets"
  editor = "code"
  gh_token = "" # used for the update feature
}

manager "aws" "default" {
  access_id = ""
  secret_key = ""
  region = ""
}

# if you have boto credentials set up with multiple
# profiles you can select that profile by setting it in the manager block
manager "aws" "testing" { 
  profile = "aws_testing"
  region = "us-east-1"
}

manager "gcp" "gcp-sandbox" {
  creds_file = "PATH/TO/YOUR/GCP/CREDENTIALS_FILE.json"
}

# to create an api key either goto the [web console for gcp](https://console.cloud.google.com/apis/credentials)
# or use the gcloud cli tool to generate one `gcloud alpha services api-keys create --display-name=KEY_NAME` make
# sure you have the right project set in the gcloud tool when generating the key
manager "gcp" "gcp-testing" {
  api_key = "gcp api key here"
}
```

You can override the default profile by passing the `--profile/-p` flag. The `secrets_path` can be set with the `--path` flag and the `editor` can be set with the `$EDITOR` environment variable.

### Securing your config file

To keep in the spirit of security jaws can encrypt your config file using `filippo.io/age`. This is for developers to keep a `jaws.conf` file locally without the risk of it being leaked from the computer, it is not to allow devs to commit the config to a repo, although there is potential use cases for this in a CICD system.

**How to use**:

After creating a config and placing it in a valid config location (see [jaws configuraion](#jaws-configuraion)), run `jaws config lock` to encrypt the file. This will prompt for a password to encrypt with. Once encrypted you can use the jaws cli like normal but it will ask for a password each time `jaws` is called so the config can be loaded into memory. In order to avoid a password prompt everytime set the env variable `JAWS_CONFIG_KEY` meaning `export JAWS_CONFIG_KEY="<SUPER-SECRET-KEY>"`. The problem with this is now the password is in your shell history as well as in your current tty session, one potential way to get around this is to use a password manager's cli tool like 1password's or bitwarden's.

**1Password**:

> If you are on a Mac you can use touchID to authenicate, and on Windows you can use Windows Hello to authenticate.

> For updated install and setup instructions go to [1Password](https://developer.1password.com/docs/cli/get-started/#install)

Deps:

- 1Password 8
- [op](https://developer.1password.com/docs/cli/get-started/#install) >= 2 (1Password cli)

Step 1. [Turn on Biometric authentication for the cli](https://developer.1password.com/docs/cli/get-started/#turn-on-biometric-unlock)

Step 2. Add new password entry into 1Password's private vault. Name it `JAWS_CONFIG_KEY`.

Step 3. Add one of the below to your shell rc file. Usually located `~/.zshrc`.

```sh
function jaws-op {
  JAWS_CONFIG_KEY=$(op item get JAWS_CONFIG_KEY --fields label=password) jaws "$@"
}
alias jaws=jaws-op
```

> A note about using the above method, it will be slow due to running the op cli everytime you call `jaws`. The second option below is faster but requires running `jaws-on` before using the cli in each terminal session. Just one more thing to remember before using the cli, comes down to user preferance.

```sh
function jaws-on {
  export JAWS_CONFIG_KEY=$(op item get JAWS_CONFIG_KEY --fields label=password)
}

function jaws-off {
  unset JAWS_CONFIG_KEY
}
```

Step 4. Either restart your terminal or run `source ~/.zshrc` to load the changes.

Step 5. Lock the config file. `jaws config lock` (first run `jaws-on` if you are using the second shell function)

If you need to edit the config you can run `jaws config edit`, which will decrypt then re-encrypt the config once you are done.

**TODO**:

- add bitwarden howto

---

## [Managing ENV files with jaws](./manage-env.md)
