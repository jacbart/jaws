[Back to Readme](../README.md)

# Environment Files

Using HCL to create a custom config, jaws can generate environment variable files and download secrets as files directly into your project. This is to help prevent the common mistake of hardcoding variables or commiting an env file with all api and security keys already in them, or worst of all commiting a env file and not providing the variable info to other developers in an easy to find location.  

---

## Vars

there are two types of variables `var.BLAH_BLAH` and `secret.BLAH_BLAH`. `var.BLAH_BLAH` is set in the `.jaws` file in a locals block or can be overwritten with an environment varilable prefixed with `JAWS_`. The second one `secret.BLAH_BLAH` comes from the filtered secrets i.e. if the filter is `testing/jaws` then `var.BLAH_BLAH` = `testing/jaws/blah-blah`.

## Functions

>all functions return a string and can be used together.

**quote**:
adds quotes to the input string.  
  ex: `quote("quote me")` -> `"quote me"`

**encode**:  
takes a single variable or string and base64 encodes it.  
  ex: `encode("encode this string")` -> `ZW5jb2RlIHRoaXMgc3RyaW5nCg==`  

**decode**:  
takes in a base64 encoded string and decodes it.  
  ex: `decode("ZW5jb2RlIHRoaXMgc3RyaW5nCg==")` -> `encode this string`  

**file**:  
writes a string or variable into a file. The function takes two args the first is the file path and the second is the contents of the file.  
  ex: `file("cert.pem", var.CERT)` creates a file in the current directory called `cert.pem` with the contents of the variable `var.CERT`  

**sh**:  
runs a command with comma separted arguments and outputs as a string. Only one argument is required for the interpretor.  
  ex: `sh("curl", "server.com")`  

**resolve**:  
takes in a FQDN and returns an IPv4 address. If there is no IPv4 address then it returns the FQDN.   
  ex: `resolve("google.com")` -> `142.250.72.174`  

**escape**:  
escape a specified character. Function take two args, the first is the character to escape and the second is the string.  
  ex: `CONFIG = {"servername": "test"}`  
  `escape("\"", var.CONFIG)` -> `{\"servername\": \"test\"}`  

**input**:  
asks the user for input and inserts the response as the value.   
  ex: `input("Set VAR:", "default value")`

## Example env config

`env.jaws`
```
msg = "this goes at the top of the output in a comment and is optional"
out = "env" # optional, if a recognized file extension is set like yaml/yml then that format is used for the output
format = "yaml" # optional if you want to force a output format (yaml, json, tfvars, env)
profile = "default" # non functional, just for info
filter = "testing/jaws/"

locals {
  env = "dev"
}

group "Group Label" {
  WHATENV_IS_THIS = quote(local.env)
}

vars { # each vars block groups those variables together
  CONFIG = encode(var.CONFIG)
  cert_file = file("cert.pem", var.CERT)
  SERVER = quote("${resolve("localhost")}:${var.PORT}")
  CMDOUT = sh("echo", ${env.MSG})
}
```

>By default jaws will backup any conflicting file set by `--out/-o`. To Disable this and instead have a prompt, use the flag `--disable-safe/-D`, or use `--overwrite/-O` to just skip and overwrite any file.

```sh
# grabs all secrets filtered by testing/jaws and outputs an .env file
jaws pull -o .env testing/jaws
# or
# if no input file is set but there is a .jaws file in the directory, jaws
# will load the file and produce a .env file or whatever the out_file field is set too
jaws pull
# load the env into you current shell session
source .env
```

---

## Syntax highlighting

### VSCode  
install the [HCL extenstion](https://marketplace.visualstudio.com/items?itemName=hashicorp.hcl) then add the below to your `settings.json`

```json
"files.associations": {
  "*.jaws": "hcl"
}
```

### Helix
Create `~/.config/helix/languages.toml` and add the below.  

```toml
[[language]]
name = "hcl"
file-types = ["tf", "tfvars", "hcl", "jaws"]
auto-format = true
```

