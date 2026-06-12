# JAWS Demo - Just A Working Secretsmanager
# Run: ./scripts/demo.sh
# Output: assets/demo.gif

Output assets/demo.gif

Require jaws

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 25ms
Set PlaybackSpeed 1.5

# -- Setup: temp workspace, disable keychain, prep secret files --
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
Show

# -- Clear the screen so setup output is gone --
Type "clear"
Sleep 100ms
Enter
Sleep 300ms

# -- Title --
Type "# JAWS - Just A Working Secretsmanager"
Sleep 500ms
Enter
Sleep 300ms

# -- Create two secrets --
Type "jaws create prod/db-password -d 'Production DB password' -f /tmp/jaws-demo-secret.txt"
Sleep 200ms
Enter
Sleep 1500ms

Type "jaws create staging/api-key -d 'Staging API key' -f /tmp/jaws-demo-api.txt"
Sleep 200ms
Enter
Sleep 1500ms

# -- List secrets --
Type "jaws list"
Sleep 200ms
Enter
Sleep 1000ms

# -- Pull and print a secret --
Type "jaws pull jaws://prod/db-password -p"
Sleep 200ms
Enter
Sleep 1000ms

# -- Template injection (create template then inject) --
Type "cat > .env.tpl <<'EOF'"
Enter
Type "DB_PASS={{jaws://prod/db-password}}"
Enter
Type "API_KEY={{jaws://staging/api-key}}"
Enter
Type "EOF"
Sleep 200ms
Enter
Sleep 500ms

Type "jaws pull -i .env.tpl"
Sleep 200ms
Enter
Sleep 1500ms

# -- Operation log --
Type "jaws log"
Sleep 200ms
Enter
Sleep 1000ms

# -- Version history --
Type "jaws log jaws://prod/db-password"
Sleep 200ms
Enter
Sleep 1000ms

# -- Cleanup --
Type "jaws clean --force"
Sleep 200ms
Enter
Sleep 1500ms

# -- Version sign-off --
Type "jaws version"
Sleep 200ms
Enter
Sleep 800ms
