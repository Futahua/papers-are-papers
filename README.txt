PAPERS ARE PAPERS — AGENT-FIRST FOUNDATION

Papers is a native Windows workspace that puts a guarded AI operator between
the creator's intent and the computer.

CURRENT TRUTH

- The application, Hermes protocol client, local state, companion, Inspect
  mode, staged self-edit services, restricted builder tools, and recovery
  launcher are real code.
- Hermes Agent is pinned to package version 0.18.0 from official release tag
  v2026.7.1 and commit 7c1a029.
- Papers does not contain Hermes WebUI and does not generate fake AI replies.
- A real Hermes installation, Nous sign-in, Windows Computer Use task, and
  AI-produced self-edit still require creator-tested end-to-end verification.
- Backpacks are deferred until this agent foundation is trustworthy enough to
  generate them from desired behavior.

RUN FOR DEVELOPMENT

From the repository root:

    npm install
    npm run tauri dev

BUILD AND TEST

    npm run check
    npm run test
    npm run build

From src-tauri:

    cargo test --all-targets --locked
    cargo build --locked --bins

From launcher:

    cargo test --locked
    cargo build --locked

The debug executables are:

    src-tauri/target/debug/papers.exe
    src-tauri/target/debug/papers-mcp.exe
    launcher/target/debug/papers-launcher.exe

The canonical product context and handoff rules are in PROJECT_CONTEXT.md.
