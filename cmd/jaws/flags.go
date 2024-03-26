package main

// Flags for the jaws cli
func Flags() {
	// global persistent flags
	rootCmd.PersistentFlags().StringVar(&secretsPath, "path", "secrets", "sets download path for secrets, overrides config")
	rootCmd.PersistentFlags().StringVarP(&cfgFile, "config", "c", "", "set config file")
	rootCmd.PersistentFlags().StringVarP(&profile, "profile", "p", "", "set current secrets manager profile as defined in your jaws.conf file")
	rootCmd.PersistentFlags().BoolVar(&debugMode, "debug", false, "set flag to print logging info")
	// config
	configCreateCmd.Flags().BoolVar(&cicdMode, "cicd", false, "set flag to disable prompts")
	// version command Flags
	versionCmd.Flags().BoolVarP(&shortVersion, "short", "s", false, "return version only")
	versionCmd.Flags().BoolVar(&checkUpdateOnly, "check", false, "check for a newer version")
	// update command flags
	updateCmd.Flags().BoolVar(&checkUpdateOnly, "check", false, "check for a newer version")
	// create command flags
	addCmd.Flags().BoolVarP(&useEditor, "editor", "e", false, "open any selected secrets in an editor")
	// delete command flags
	deleteCmd.Flags().BoolVarP(&deleteCancel, "cancel", "C", false, "[AWS] cancel a secret scheduled for deletion")
	// pull command flags
	pullCmd.Flags().BoolVar(&print, "print", false, "print secret string to terminal instead of downloading to a file [direct secrets flag]")
	pullCmd.Flags().BoolVarP(&useEditor, "edit", "e", false, "open any selected secrets in an editor [direct secrets flag]")
	pullCmd.Flags().StringVarP(&inputEnvFile, "in", "i", "", "set a jaws env config file, overrides the prefix flag [env file flag]")
	pullCmd.Flags().StringVarP(&outputEnvFile, "out", "o", ".env", "set output file for the env [env file flag]")
	pullCmd.Flags().BoolVar(&diffEnv, "diff", false, "show diff of current env file if it already exists [env file flag]")
	pullCmd.Flags().BoolVarP(&overwriteEnv, "overwrite", "O", false, "overwrite old env file without prompt [env file flag]")
	pullCmd.Flags().BoolVarP(&disabledSafeEnv, "disable-safe", "S", false, "set flag to turn off safe mode to prevent backups of any conflicting env file before writing the new file")
	pullCmd.Flags().BoolVar(&recursiveSearch, "R", false, "recursively check for .jaws files - NOT IMPLEMENTED YET")
	pullCmd.Flags().StringVarP(&outFormat, "format", "f", "", "set output format type, only use if output file is not set. Options: yaml, json, tfvars")
	pullCmd.Flags().BoolVarP(&disableDetectJawsFiles, "disable-auto-detect", "A", false, "set to false to force secrets to be pulled instead of using the jaws file in the directory")
	pullCmd.Flags().StringVarP(&envFilter, "filter", "F", "", "filter override for the env manager")
	// push command flags
	pushCmd.Flags().BoolVar(&createPrompt, "disable-prompt", false, "add this flag to skip the confirmation prompt of new secrets")
	pushCmd.Flags().BoolVarP(&cleanLocalSecrets, "keep", "k", false, "set to keep secrets after pushing/setting them")
}
