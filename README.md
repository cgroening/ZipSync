# ZipSync

ZipSync is a Rust-based CLI tool for synchronizing files and directories with ZIP archives. It supports both creating ZIP files from local sources and extracting content from ZIP files to the local filesystem, all based on a flexible JSON configuration file.

## Features

- Copy files and directories into ZIP archives (`to_zip` mode)
- Extract content from ZIP archives to disk (`from_zip` mode)
- Flexible target paths inside the archive
- Optional user confirmations for overwrites, creation, and deletions

## Usage

1. Make sure Rust is installed:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Create a configuration file at `data/config.json` (see structure below).

4. Run the tool:
   ```bash
   cargo run --release
   ```

## Configuration File (`data/config.json`)

Zipsync requires a JSON configuration file with the following structure:

```json
{
  "zip_path": "/path/to/your.zip",
  "direction": "to_zip",
  "confirm_new": "yes",
  "confirm_overwrite": "yes",
  "confirm_delete": "yes",
  "copy_tasks": [
    {
      "description": "Short task description",
      "source": "/path/to/source/file_or_folder",
      "target": "target/path/in/zip/"
    }
  ]
}
```

### Fields:

- `zip_path`: Path to the ZIP file to be read or written.
- `direction`: Either `"to_zip"` or `"from_zip"`.
- `confirm_new`: `"yes"` or `"no"` – prompt before creating new files.
- `confirm_overwrite`: `"yes"` or `"no"` – prompt before overwriting existing files.
- `confirm_delete`: `"yes"` or `"no"` – prompt before deleting files.
- `copy_tasks`: An array of copy operations:
  - `description`: A short description of the task.
  - `source`: Path to the source file or directory.
  - `target`: Destination path within the ZIP file (empty string `""` for root).

## Example

```json
{
  "zip_path": "backup.zip",
  "direction": "to_zip",
  "confirm_new": "yes",
  "confirm_overwrite": "yes",
  "confirm_delete": "yes",
  "copy_tasks": [
    {
      "description": "Copy file to ZIP root",
      "source": "/home/user/file.txt",
      "target": ""
    },
    {
      "description": "Copy folder into subfolder of ZIP",
      "source": "/home/user/docs/",
      "target": "documents/"
    }
  ]
}
```
