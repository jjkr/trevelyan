# Trevelyan

Trevelyan is a native AI agent for macOS, built with Tauri and Rust. It provides a simple, keyboard-centric interface for interacting with AI models and can be extended with tools via the Model Context Protocol (MCP).

## Features

*   **Global Hotkey:** Access Trevelyan from anywhere with a configurable keyboard shortcut (default: `Super + Space`).
*   **Modern UI:** A clean, minimal interface with a blurred background effect.
*   **Extensible:** Supports MCP servers for adding new tools and capabilities.

## Getting Started

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   [Node.js](https://nodejs.org/en/download/) (for frontend development, if you choose to use a framework)

### Installation

1.  Clone the repository:
    ```sh
    git clone https://github.com/your-username/trevelyan.git
    ```
2.  Navigate to the project directory:
    ```sh
    cd trevelyan
    ```
3.  Run the application in development mode:
    ```sh
    cd src-tauri
    cargo tauri dev
    ```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
