# JAWS Demo: Creating Secrets
# Output: assets/demo-create.gif

Output assets/demo-create.gif

Require jaws

Set Shell "bash"
Set FontSize 14
Set Width 900
Set Height 420
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 30ms
Set PlaybackSpeed 1.2

# -- Hidden setup --
Hide
Type "cd $(mktemp -d) && export EDITOR=cat && export PS1='$ '"
Enter
Sleep 300ms
Type "jaws config init --minimal --path ./jaws.kdl && jaws config set keychain_cache false"
Enter
Sleep 1500ms
Type "echo 'super-secret-db-password-123' > /tmp/jaws-demo-secret.txt && echo 'sk-abc123-staging-api-key' > /tmp/jaws-demo-api.txt"
Enter
Sleep 200ms
Type "clear"
Enter
Sleep 300ms
Show

# -- Create two secrets --
Type "jaws create prod/db-password -d 'Production DB password' -f /tmp/jaws-demo-secret.txt"
Sleep 200ms
Enter
Sleep 2000ms

Type "jaws create staging/api-key -d 'Staging API key' -f /tmp/jaws-demo-api.txt"
Sleep 200ms
Enter
Sleep 2000ms

# -- List them --
Type "jaws list"
Sleep 200ms
Enter
Sleep 1500ms
