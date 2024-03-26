package envmanager

import (
	b64 "encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
	"github.com/jacbart/jaws/utils/tui"
	"github.com/zclconf/go-cty/cty"
	"github.com/zclconf/go-cty/cty/function"
	"gopkg.in/yaml.v2"
)

// contextFuncs - returns a map of functions for hcl context
func contextFuncs() map[string]function.Function {
	return map[string]function.Function{
		"unquote": function.New(&function.Spec{ // unquote(content)
			Params:   []function.Parameter{},
			VarParam: &function.Parameter{Type: cty.String},
			Type:     function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				out := make([]string, 0, len(args))
				for _, s := range args {
					out = append(out, s.AsString())
				}
				outStr := strings.Join(out, " ")

				if len(outStr) > 0 && outStr[0] == '"' {
					outStr = outStr[1:]
				}
				if len(outStr) > 0 && outStr[len(outStr)-1] == '"' {
					outStr = outStr[:len(outStr)-1]
				}
				return cty.StringVal(outStr), nil
			},
		}),
		"quote": function.New(&function.Spec{ // quote(content)
			Params:   []function.Parameter{},
			VarParam: &function.Parameter{Type: cty.String},
			Type:     function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				out := make([]string, 0, len(args))
				for _, s := range args {
					out = append(out, s.AsString())
				}
				outStr := strings.Join(out, " ")

				returnStr := fmt.Sprintf("%q", outStr)
				return cty.StringVal(returnStr), nil
			},
		}),
		"encode": function.New(&function.Spec{ // encode(content)
			Params:   []function.Parameter{},
			VarParam: &function.Parameter{Type: cty.String},
			Type:     function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				out := make([]string, 0, len(args))
				for _, s := range args {
					out = append(out, s.AsString())
				}
				outStr := strings.Join(out, " ")

				encoded := b64.StdEncoding.EncodeToString([]byte(outStr))
				return cty.StringVal(encoded), nil
			},
		}),
		"decode": function.New(&function.Spec{ // decode(content)
			Params:   []function.Parameter{},
			VarParam: &function.Parameter{Type: cty.String},
			Type:     function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				out := make([]string, 0, len(args))
				for _, s := range args {
					out = append(out, s.AsString())
				}
				outStr := strings.Join(out, " ")

				decodeBytes, err := b64.StdEncoding.DecodeString(outStr)
				if err != nil {
					return cty.NilVal, err
				}
				return cty.StringVal(string(decodeBytes)), nil
			},
		}),
		"file": function.New(&function.Spec{ // file(file, content)
			Params: []function.Parameter{
				{Type: cty.String, Name: "file"},
				{Type: cty.String, Name: "content"},
			},
			Type: function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				// set file
				cFile := args[0]
				file := cFile.AsString()

				// set file content
				cContent := args[1]
				content := cContent.AsString()

				_, err := os.Stat(file)
				if err == nil {
					err = os.Remove(file)
					if err != nil {
						return cty.NilVal, err
					}
					fmt.Printf("%s %s\n", style.ChangedString("replacing"), file)
				} else if errors.Is(err, os.ErrNotExist) {
					fmt.Printf("%s %s\n", style.InfoString("creating"), file)
				} else {
					return cty.NilVal, err
				}

				base := filepath.Base(file)
				path := strings.TrimSuffix(file, base)

				log.Default().Printf("envmanager: file function path=%s file=%s\n", path, base)

				if path != "" {
					err = os.MkdirAll(path, os.ModePerm)
					if err != nil {
						return cty.NilVal, err
					}
				}

				// create file
				f, err := os.Create(file)
				if err != nil {
					return cty.NilVal, err
				}
				defer f.Close()

				// write to file
				_, err = f.WriteString(content)
				if err != nil {
					return cty.NilVal, err
				}

				return cty.StringVal(FILE_FUNC_SUCCESS + file), nil
			},
		}),
		"resolve": function.New(&function.Spec{ // resolve(hostname)
			Params: []function.Parameter{
				{Type: cty.String, Name: "hostname"},
			},
			Type: function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				if len(args) > 1 {
					return cty.NilVal, errors.New("resolve only accepts one FQDN")
				}
				fqdn := args[0].AsString()
				if !validateDomainName(fqdn) && fqdn != "localhost" {
					return cty.NilVal, fmt.Errorf("invalid FQDN: %s", fqdn)
				}
				addr, err := net.LookupIP(fqdn)
				if err != nil {
					return cty.NilVal, fmt.Errorf("unknown FQDN: %s", fqdn)
				}
				retAddr := fqdn
				for _, ip := range addr {
					if ip.To4() != nil {
						retAddr = ip.String()
					}
				}
				return cty.StringVal(retAddr), nil
			},
		}),
		"escape": function.New(&function.Spec{ // escape(char, content)
			Params: []function.Parameter{
				{Type: cty.String, Name: "char"},
				{Type: cty.String, Name: "content"},
			},
			Type: function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				if len(args) != 2 {
					return cty.NilVal, errors.New("escape needs two arguments")
				}

				char := args[0].AsString()
				content := args[1].AsString()

				content = strings.ReplaceAll(content, char, "\\"+char)
				return cty.StringVal(content), nil
			},
		}),
		"sh": function.New(&function.Spec{ // sh()
			Params:   []function.Parameter{},
			VarParam: &function.Parameter{Type: cty.String},
			Type:     function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				if len(args) < 1 {
					return cty.NilVal, errors.New("sh needs at least one argument")
				}
				var cmdArgs []string
				firstArgs, args := strings.Fields(args[0].AsString()), args[1:]
				command, firstArgs := firstArgs[0], firstArgs[1:]

				for _, arg := range firstArgs {
					cmdArgs = append(cmdArgs, arg)
				}

				for _, argStr := range args {
					argsStr := strings.Fields(argStr.AsString())
					for _, arg := range argsStr {
						cmdArgs = append(cmdArgs, arg)
					}
				}

				output, err := utils.RunCommand(command, cmdArgs)
				if err != nil {
					return cty.NilVal, err
				}
				output = strings.TrimSuffix(output, "\n")

				return cty.StringVal(output), nil
			},
		}),
		"input": function.New(&function.Spec{ // input("Description of input", "default value", width) asks for user input with a description
			Params: []function.Parameter{
				{Type: cty.String, Name: "description"},
			},
			VarParam: &function.Parameter{Type: cty.String},
			Type:     function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				var err error
				len := len(args)
				if len > 3 {
					return cty.NilVal, errors.New("input accepts 3 arguments max: (description, placeholder/default, width:defaults to 50)")
				}

				desc := ""
				placeholder := ""
				w := -1
				if len > 0 {
					desc = args[0].AsString()
				}
				if len > 1 {
					placeholder = args[1].AsString()
				}
				if len > 2 {
					i64, err := strconv.ParseInt(args[2].AsString(), 10, 32)
					if err != nil {
						return cty.NilVal, err
					}
					w = int(i64)
				}

				vars := []tui.ModelVars{
					{
						Description: desc,
						Placeholder: placeholder,
						Width:       w,
					},
				}
				res, err := tui.InputTUI(vars)
				if err != nil {
					return cty.NilVal, err
				}

				return cty.StringVal(res[0]), nil
			},
		}),
		"extract": function.New(&function.Spec{ // extract("json or yaml", "key")
			Params: []function.Parameter{
				{Type: cty.String, Name: "content"},
				{Type: cty.String, Name: "key"},
			},
			Type: function.StaticReturnType(cty.String),
			Impl: func(args []cty.Value, retType cty.Type) (cty.Value, error) {
				l := len(args)
				if l < 2 {
					return cty.NilVal, errors.New("not enough args, need 2")
				} else if l > 2 {
					return cty.NilVal, errors.New("too many args, need 2")
				}
				content := args[0].AsString()
				key := args[1].AsString()
				var value string

				if isJSON(content) {
					// process json
					var js map[string]string
					err := json.Unmarshal([]byte(content), &js)
					if err != nil {
						return cty.NilVal, err
					}
					value = js[key]
				} else if isYAML(content) {
					// process yaml
					var yml map[string]string
					err := yaml.Unmarshal([]byte(content), &yml)
					if err != nil {
						return cty.NilVal, err
					}
					value = yml[key]
				} else {
					return cty.NilVal, errors.New("unknown content type, only json and yaml supported")
				}

				return cty.StringVal(value), nil
			},
		}),
	}
}

// validateDomainName takes in a domain string and return true if it is valid and false if not
func validateDomainName(domain string) bool {
	RegExp := regexp.MustCompile(`^(([a-zA-Z]{1})|([a-zA-Z]{1}[a-zA-Z]{1})|([a-zA-Z]{1}[0-9]{1})|([0-9]{1}[a-zA-Z]{1})|([a-zA-Z0-9][a-zA-Z0-9-_]{1,61}[a-zA-Z0-9]))\.([a-zA-Z]{2,6}|[a-zA-Z0-9-]{2,30}\.[a-zA-Z
]{2,3})$`)
	return RegExp.MatchString(domain)
}
