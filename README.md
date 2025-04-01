# EPUB Reader

A terminal-based EPUB reader written in Rust. This application allows you to read EPUB files directly in your terminal with a clean, navigable interface.

## Features

- Open and read EPUB 2.0 and 3.0 format books
- Terminal-based user interface with vim-like navigation
- Chapter navigation (next/previous/goto)
- Table of contents view
- Metadata display
- Text rendering with basic formatting (headings, paragraphs, emphasis)
- Keyboard shortcuts for easy navigation

## Installation

### Prerequisites

- Rust and Cargo (install from [rustup.rs](https://rustup.rs/))

### Building from Source

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/epub_reader.git
   cd epub_reader
   ```

2. Build the project:
   ```
   cargo build --release
   ```

3. The executable will be available at `target/release/epub_reader`

## Usage

Run the application with an EPUB file as an argument:

```
epub_reader path/to/your/book.epub
```

## Navigation and Commands

### Keyboard Shortcuts (Normal Mode)

- `j`: Scroll down
- `k`: Scroll up
- `Ctrl+d`: Scroll half page down
- `Ctrl+u`: Scroll half page up
- `g`: Go to the beginning of the text
- `G`: Go to the end of the text
- `n`: Go to the next chapter
- `p`: Go to the previous chapter
- `:`: Enter command mode
- `q`: Quit the application
- `Esc`: Return to main view from TOC or metadata view

### Command Mode

Enter command mode by pressing `:` and then type one of the following commands:

- `q` or `quit`: Exit the application
- `n` or `next`: Go to the next chapter
- `p` or `prev`: Go to the previous chapter
- `g <number>` or `goto <number>`: Go to a specific chapter by number
- `t` or `toc`: Show the table of contents
- `m` or `meta`: Show the book metadata

Press `Enter` to execute a command or `Esc` to cancel.

## Project Structure

- `src/main.rs`: Application entry point
- `src/epub/mod.rs`: EPUB file parsing and handling
- `src/navigation.rs`: Chapter navigation and TOC management
- `src/metadata.rs`: EPUB metadata handling
- `src/render/mod.rs`: XHTML to text rendering
- `src/ui/mod.rs`: Terminal UI implementation
- `src/errors.rs`: Error handling

## Dependencies

- `zip`: For handling EPUB files (which are ZIP archives)
- `roxmltree`: XML parsing
- `scraper`: HTML parsing
- `thiserror`: Error handling
- `ego-tree`: Tree data structures
- `ratatui`: Terminal UI framework
- `crossterm`: Terminal manipulation
- `unicode-width`: Unicode text width calculations
- `textwrap`: Text wrapping utilities

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
