# JAWS

Just A Working Secretsmanager

The goal of jaws is to be a easy to pick up tool for any dev familiar with git. Once the
config/authentication is setup jaws should use commands similar to git i.e. `jaws pull` `jaws push`
`jaws add PROD/APP/Key` `jaws restore PROD/APP/Item` `jaws rm PROD/APP/Key`

## TODO

- [ ] Local Version control for secrets
- [ ] Push/upload changed or new secret
- [ ] Local Encyption for secrets and config
  - passphrase
  - ssh key
  - hardware key/device id
- [ ] api, interface for external service interacting similar to hashivault
