# Trevelyan

Trevelyan is a native AI agent for macOS, built with Tauri and Rust. It provides a simple, keyboard-centric interface for interacting with AI models and can be extended with tools via the Model Context Protocol (MCP).

## Features

*   **Global Hotkey:** Access Trevelyan from anywhere with a configurable keyboard shortcut (default: `Super + Space`).
*   **Modern UI:** A clean, minimal interface with a blurred background effect, now powered by **React and Vite**.
*   **Extensible:** Supports MCP servers for adding new tools and capabilities.

## Getting Started

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   [Node.js](https://nodejs.org/en/download/) (for frontend development)

### Installation

1.  Clone the repository:
    ```sh
    git clone https://github.com/your-username/trevelyan.git
    ```
2.  Navigate to the project directory:
    ```sh
    cd trevelyan
    ```
3.  Install Node.js dependencies for the Tauri project:
    ```sh
    npm install
    ```
4.  Navigate to the frontend directory and install its dependencies:
    ```sh
    cd frontend
    npm install
    cd ..
    ```
5.  Run the application in development mode:
    ```sh
    npm run tauri dev
    ```

### Frontend Development

The frontend is built with React and Vite. You can find the source code in the `frontend/` directory.

*   **Development Server:** To run the frontend development server independently (without the Tauri wrapper), navigate to the `frontend/` directory and run `npm run dev`.
*   **Build:** To build the frontend for production, navigate to the `frontend/` directory and run `npm run build`.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
