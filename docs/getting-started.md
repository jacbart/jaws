[Back to Readme](../README.md)

# Getting Started

## Install jaws  

go to the releases page of [jaws](https://github.com/jacbart/jaws/releases) and download the tar file for your computer. Then install the binary using the commands below. Or just open the archive and move the jaws binary to your PATH.  

```sh
TMPDIR="$(mktemp -d)"
tar -xf ~/Downloads/jaws_*.tar.gz -C $TMPDIR
mv $TMPDIR/jaws ~/.local/bin
jaws version # check it is in your PATH and working
# If you are on macOS you might need to go into Security & Privacy to allow the executable
rm -rf $TMPDIR
```

For other install options refer to the [Install Guide](./install.md)  

## Create Config for jaws

```sh
jaws config create > jaws.conf
```

### Auth

#### AWS

There are three ways to authenticate jaws with AWS.

1. filling in the manager block in the `jaws.conf` file.  
```hcl
manager "aws" "default" {
  access_id = "AWS ACCESS ID"
  secret_key = "AWS SECRET KEY"
  region = "us-east-1"
}
```

2. Setup the boto3 aws credential files, by creating `~/.aws/credentials` and `~/.aws/config`.  

`credentials`
```ini
[default]
aws_access_key_id = <AWS ACCESS ID>
aws_secret_access_key = <AWS SECRET KEY>
```

`config`
```ini
[default]
region = us-east-1
output = json
```

3. The last way is also using boto3 but instead set everything with env variables.  
```sh
export AWS_ACCESS_KEY_ID="AWS ACCESS ID"
export AWS_SECRET_ACCESS_KEY="AWS SECRET KEY"
export AWS_REGION="us-east-1"
```

> For more info on your config file goto [Configure Secrets Manager](./configure.md)

## Manage a project's environment variable file

In the root of your project, create a file ending in `.jaws`. Copy the below example as a starting place.

```
msg = "Example env file - Managed By Jaws"
out = "example.env"
profile = "default"

vars {
   HELLO_WORLD = "Hello World"
   LOCALHOST = resolve("localhost")
   HOSTNAME = command("hostname")
}
```

---

## [Managing ENV files with jaws](./manage-env.md)  
