# Spec Delta

## MODIFIED Requirements

### Requirement: Client reload
A client SHALL reload `[client]` when the config file is saved, including when an editor replaces the file by rename and when the path is a symlink. The `client.config.reload` action SHALL also reload it.

#### Scenario: Save while attached
- **WHEN** a user adds `"alt-y" = { action = { server.tab.create = {} } }` under `[client.modes.normal.keys]` and saves while a client is attached
- **THEN** `alt-y` creates a tab without a restart, and `alt-n` still works

#### Scenario: Rename-save through a symlink
- **WHEN** the config path is a symlink into another directory and the target is saved by an editor that writes a new file and renames it into place
- **THEN** the attached client applies the change
