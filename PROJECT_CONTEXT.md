# Papers are Papers - Project Context and Handoff Rules

Last updated: 2026-07-07
Canonical local repository: `C:\This is Minh\LapSlop brotherhood\Programs\Papers are papers\REAL`  
GitHub: `https://github.com/Futahua/papers-are-papers`  
Visibility: Private  
Default branch: `main`

## Why this document exists

This is the canonical plain-language context for Papers are Papers.

It exists to prevent future agents from:

- Mistaking an old proof of concept for the intended product.
- Treating an idea as if it has already been implemented.
- Repeating architectural conversations that have already been settled.
- Inventing technical systems before the desired behavior is understood.
- Overstating the quality, completeness, safety, or readiness of the app.
- Forcing a non-technical creator to make implementation decisions.

Every future agent should read this document before planning or changing the project.

## The creator

The creator does not code. They are responsible for the ideas, desired behavior, references, taste, and final judgment of whether the experience feels right.

They can:

- Explain what they want through examples and analogies.
- Reference behaviors from existing programs.
- Test an application as a user.
- Recognize when a result feels right, wrong, slow, confusing, or artificial.
- Decide what the product should mean.

They should not be expected to:

- Select frameworks, libraries, databases, or internal patterns.
- Translate ideas into developer terminology.
- Diagnose implementation problems.
- Write formal technical specifications alone.

The agent is responsible for translating intent into clear behavior, making technical decisions, explaining meaningful tradeoffs in plain language, implementing carefully, and verifying claims with evidence.

## The product in one sentence

Papers are Papers is a universal, AI-native personal workspace layered over Windows, where the user enters different "Backpacks" to interact with their real files, knowledge, programs, tools, and work without trapping them inside a new system.

## What kind of application this is

Papers is best described as an AI-native personal computing environment or personal meta-OS.

It is not literally a replacement operating system. Windows still owns hardware, security, processes, and the filesystem. Papers is intended to become the personal layer through which the creator interacts with those capabilities.

It combines aspects of:

- A desktop shell.
- A universal launcher.
- A second brain or Life OS.
- A knowledge-management environment.
- An automation cockpit.
- A global AI-agent interface.
- A host for flexible, purpose-specific workspaces.

The word "universal" currently means broad access across the creator's data, programs, and ways of working. It does not yet mean that the app must run on every operating system.

Windows is the current target.

## The central philosophy

Papers should not become another isolated program that performs a closed set of actions.

It should feel like a lightweight abstraction layer over the creator's existing machine, databases, files, programs, and work.

The creator described it as:

- A hub for playing with their machine, databases, and work.
- A background layer that uses the capability of the machine already in their hands.
- A virtual notebook with different toys.
- A way to avoid migrating to a different operating system.
- A system with as little overhead, bloat, fluff, and unnecessary design as possible.

The product must not gain apparent power by hiding where information really lives.

## Current product vocabulary

### Basic

Basic is the permanent menu control, not a Backpack and not a home screen.

Its known top-level destinations are:

- Backpacks.
- Tools.
- Settings.

The permanent shell should remain understandable regardless of which Backpack is active.

### Backpack

A Backpack is a room or environment for a particular way of interacting with the creator's things.

A Backpack is not:

- A sealed application.
- A data silo.
- A mutually exclusive database.
- A container that owns the underlying files.

Different Backpacks may:

- Use the same files.
- Share tools.
- Overlap in purpose.
- Present the same information differently.
- Contain multiple pages, views, features, and tool calls.

The creator enters whichever Backpack suits the current activity, like reaching into a real backpack for what is needed.

When Papers launches and Backpacks exist, the current expectation is that the last active Backpack returns by default. Basic and the global AI control remain available over it.

### Tool

A Tool is a capability that works across the system.

Examples may include:

- Installed programs.
- Shortcuts.
- Scripts.
- Automation helpers.
- Mounted discs or locations.
- Synchronization.
- Machine utilities.

Tools may be enabled or disabled and may persist across different Backpacks. They should not be unnecessarily locked to one Backpack.

The exact Tool contract has not yet been defined.

### Global AI

The AI is intended to become one of the main ways the creator works with their things. It is not merely a side chatbot.

The AI should eventually:

- Understand the creator's broad data world, not only the active Backpack.
- Understand Papers itself, including Backpacks and Tools.
- Know where organized material really lives when asked.
- Use existing, capable agent harnesses, memories, and purpose-built pipelines where appropriate.
- Avoid reinventing agent infrastructure merely for the sake of owning it.
- Admit uncertainty and ask a useful follow-up when it genuinely does not understand.

**Partial:** Papers has now installed its pinned Hermes Agent 0.18.0 runtime on the creator's machine, reached a healthy loopback server, installed a hash-verified private Cua Driver 0.7.0, launched the latest release build successfully, completed Nous Portal sign-in, and completed real model turns through the Papers app bridge. The working free model is currently `stepfun/step-3.7-flash:free` through Nous Portal. A harmless inspect-only Computer Use pass against Notepad succeeded without editing. A creator-approved low-risk Windows action also succeeded: Hermes typed `PAPERS_TYPED_OK` into a disposable Notepad scratch tab and did not save the file. Pause/Stop behavior, self-edit preview, Keep/Reject, GitHub sync from self-edit, and rollback remain unverified, so no documentation may describe the agent as end-to-end working yet.

### Data source

A Data Source is any real location or system Papers can understand or work with.

This concept is not yet fully defined. It may eventually include folders, files, databases, programs, services, or other machine-accessible information.

Papers should point back to real sources rather than pretending that imported copies are the originals.

## Product laws

These principles should be treated as stronger than individual features unless the creator explicitly changes them:

1. The creator's real files and systems remain authoritative.
2. Backpacks must not become artificial information silos.
3. The global AI is not limited to the active Backpack unless the creator asks for that limitation.
4. Tools can be reused across Backpacks.
5. The permanent shell must always provide a clear way to navigate and recover.
6. The app should reveal where information actually lives when asked.
7. The product should feel lightweight even when its capabilities become broad.
8. Useful results should appear progressively instead of freezing the interface.
9. Long operations should remain visible, cancellable, and non-blocking.
10. Significant AI actions should be understandable and reversible.
11. The app should not require the creator to become technical.
12. The system should reuse strong existing agent infrastructure rather than rebuild it without a product reason.
13. No documentation may claim a feature works when it is only visual, mocked, planned, or partially connected.
14. Performance and experience are both product requirements.
15. The creator's current instruction overrides older documents, prototypes, and assumptions.

## Current priority and deferred first Backpack

**Superseded:** manually designing the Life OS Backpack is no longer the first implementation target.

**Confirmed decision:** the first priority is a solid global AI operator and inline editor. The creator intends to rely on that agent to create and revise future Backpacks through experience-driven requests rather than requiring a human or coding agent to pre-design every Backpack.

**Confirmed correction:** the near-term product should feel like a practical agent workbench, closer to Codex or Claude Code than to a broad observability dashboard. Do not prioritize multi-agent collaboration, grand research automation, or a full "living organism" interface now. The mundane UX matters first: streaming answers, concise public reasoning summaries, visible tool steps, files/artifacts, diffs, approvals, and self-edit previews.

**Deferred:** the first generated Backpack is still expected to be a knowledge-management, second-brain, or Life OS Backpack once the agent foundation has earned enough trust.

The first Life OS Backpack should eventually help the creator:

- Capture something quickly.
- Retrieve information naturally.
- Reveal useful relationships.
- Synthesize knowledge.
- Turn knowledge into actions or useful views.
- Work with notes, web material, embedded references, and recursive or generated views.

Its actual opening view and detailed behavior are not yet settled.

It must not become a generic Notion clone merely because Notion is a familiar reference.

## Reference behaviors

The creator may use many existing programs to communicate desired behavior. References should be interpreted narrowly and precisely.

For each reference, record:

- The program.
- The exact moment or behavior.
- Why it feels right.
- What Papers should borrow.
- What Papers should avoid.
- How that principle could appear in Papers.

Examples already discussed:

### WizTree

Possible lessons:

- Useful results appear quickly while scanning continues.
- A large amount of information can become understandable spatially and hierarchically.

Borrow responsiveness and progressive disclosure. Avoid overwhelming technical density.

### LibreOffice

Possible lessons:

- Documents feel familiar, durable, saveable, and owned by the user.
- Undo and predictable desktop conventions create trust.

Borrow predictability and ownership. Avoid toolbar overload and accumulated complexity.

### Blender, Krita, and professional creative software

Possible lessons:

- Powerful workspaces can remain navigable through stable controls and modes.
- Long operations show progress without freezing the whole application.
- Non-destructive editing, previews, and history create confidence.

Borrow workspace power and visible state. Avoid unnecessary expert jargon and setup burden.

### Raycast

Possible lesson:

- A fast universal entry point can combine navigation and action.

### Cursor

Possible lesson:

- AI changes can be proposed, inspected, accepted, rejected, or revised.

### Obsidian

Possible lesson:

- Direct visibility of user-owned files creates trust.

Do not summarize the product as a mixture of these applications. Each reference contributes only specific behavior.

## Where the project came from

The project began with a one-shot proof of concept. That experiment helped the creator make the idea visible, but it also blurred the difference between genuine behavior, visual simulation, and long-term product intent.

The original concept emphasized AI-generated "Experiences." Through later clarification, the broader Backpack model emerged:

- Backpacks are persistent rooms or lenses.
- They can overlap and use shared information.
- Tools can persist across Backpacks.
- The AI is global rather than trapped inside one room.
- Papers is a personal layer over the existing machine, not merely a host for generated mini-apps.

This history matters only because it explains why the current repository is a deliberate clean restart. Obsolete experiments and their local files are not part of the canonical project and should not appear in routine documentation or handoffs.

## Current canonical implementation

The canonical codebase is:

`C:\This is Minh\LapSlop brotherhood\Programs\Papers are papers\REAL`

The private GitHub repository is:

`https://github.com/Futahua/papers-are-papers`

The repository currently contains:

- A real Tauri Windows desktop application.
- A React and TypeScript interface compiled by Vite and embedded in Tauri.
- A Rust-native host responsible for local state, Hermes lifecycle, permissions, staged changes, and recovery records.
- A native Rust WebSocket bridge for Hermes Agent's authenticated TUI Gateway served by `hermes serve`; React receives typed frames through Tauri events and never receives the gateway token.
- A pinned Hermes Agent 0.18.0 runtime definition tied to official tag `v2026.7.1` and commit `7c1a029`.
- A verified-download installer path using the pinned official PowerShell installer hash.
- A pinned, hash-verified Cua Driver 0.7.0 archive extracted privately under Papers without PATH changes, auto-start, or Administrator access.
- A restricted Hermes installer environment that hides Windows package-manager shims, so optional tools cannot silently install outside Papers.
- A separate compact companion window and global shortcut registration.
- A generated Inspect manifest that maps rendered interface elements back to source locations.
- A guarded builder MCP executable restricted to Papers staging and unable to modify the protected trust boundary.
- SQLite tables for Papers conversations, streamed events, approvals, self-edit records, and pending GitHub synchronization.
- Git-worktree services for temporary self-edits, builds, previews, acceptance, rejection, and source rollback.
- A separate protected recovery launcher crate that starts the active healthy version and falls back to the previous one.
- A deterministic placeholder application icon.
- Locked frontend, native-app, and launcher dependencies.
- Generated Tauri schemas.

The presence of React, HTML, and CSS inside Tauri does not make this a website. They draw the interface inside a real desktop application. Machine access, background work, installation, source changes, Git operations, and recovery remain in the native layer.

### What has been verified

- The React/TypeScript source passes strict type-checking.
- The frontend production bundle builds and emits an Inspect source manifest.
- The frontend Inspect metadata test passes.
- All Rust targets compile, including Papers and the guarded MCP server.
- Native tests pass for local session state, approval policy, and protected-path rejection.
- The separate recovery launcher compiles and its healthy-version selection test passes.
- A self-contained optimized Windows release built and rendered the real first-run interface. A raw debug executable alone is not a valid visual smoke test because it expects Vite's development server.
- A shortcut collision with an older Papers process was reproduced; the new app now continues safely without the shortcut instead of crashing.
- The main interface, Inspect selection experience, and exact-size companion were visually inspected in a local browser render.
- The frontend dependency audit reports zero known vulnerabilities after updating Tauri, Vite, and Vitest to compatible patched versions.
- Papers rejected an incorrect Hermes installer checksum and executed nothing. The immutable official installer was independently verified, the lock was corrected, and the guard then accepted it.
- Papers installed Hermes Agent 0.18.0, checked out the pinned commit, verified locked Python dependencies and baseline imports, and reached a healthy loopback `hermes serve`.
- Failed Hermes partial installs were preserved, and a Papers-only Git configuration fixed Windows line-ending changes without modifying the creator's Git settings.
- The private Cua Driver executable reports version 0.7.0 and matches the official release archive digest.
- A disposable native-client smoke test authenticated with a per-launch token and received Hermes' real `gateway.ready` event without exposing a browser Origin.
- All current Rust tests, the frontend Inspect test, strict type-checking, and the final self-contained release build pass.
- The repository is private on GitHub.
- Build output under `src-tauri/target` is ignored by Git.

### What the current app honestly does

- Opens a real desktop window.
- Presents the Agent-first main workspace with conversations, a Work rail, approvals, clarification prompts, and a global Ask composer.
- Separates normal chat messages from public work items: reasoning summaries, tool steps, artifacts, approvals, and self-edit candidates.
- Truthfully shows whether Hermes is absent, stopped, starting, ready, or failed.
- Can download only the pinned official Hermes installer, verify its SHA-256 hash, and install it into Papers' private application-data directory.
- Can start and stop `hermes serve` on a random loopback-only port. The native host owns the authenticated WebSocket and forwards frames to React through Tauri events.
- Implements real Hermes session creation, prompt submission, streaming messages, tool events, clarification responses, approvals, and interruption.
- Opens a compact companion through `Ctrl+Alt+Space` when that shortcut is available; the companion can target the current window, accept a request, pause, stop, and expand Papers.
- Lets the creator enter Inspect mode, click a rendered Papers element, and describe a temporary self-change using real element and source metadata.
- Persists Papers-specific state locally in SQLite without duplicating Hermes' own conversation memory.
- Creates isolated Git worktrees for self-edit records and exposes restricted builder tools within the staging root.
- Can build, launch, accept, reject, and roll back staged source versions through explicit native commands.
- Keeps the protected runtime, permission policy, MCP boundary, Hermes lock, and launcher out of the builder's writable surface.

### What the current app does not do

- Create, open, save, or switch Backpacks.
- Present a complete Settings experience.
- Start with Windows.
- Use a system tray.
- Install itself.
- Automatically update its protected launcher or trust boundary.
- Guarantee control of elevated Administrator windows.
- Type passwords, payment details, or two-factor codes.
- Index the whole machine.
- Provide production security.
- Prove startup, memory, computer-control, or interaction performance at real scale.
- Silently install or auto-start Computer Use outside Papers.

The following have not yet been end-to-end verified on the creator's machine:

- The latest native gateway bridge launched against the installed Hermes server.
- Nous Portal OAuth and model selection.
- Hermes Computer Use operation against Windows programs.
- A real model response streamed into Papers.
- A real self-edit produced by Hermes, experienced by the creator, accepted, pushed, and rolled back.
- Offline GitHub retry and remote-divergence recovery.

Those paths contain real implementation, not canned responses, but they remain **Partial** until exercised with the external runtime and creator.

## Current architectural assessment

The current app is an Agent-first vertical-slice foundation. It is substantially beyond the empty native seed, but it is not a complete or production-safe personal operating layer.

What is directionally correct:

- Windows desktop application.
- Tauri as a lightweight native host.
- Rust available for machine-level responsibilities.
- A flexible interface layer.
- A narrow protected trust boundary for self-edits.
- Independent clean repository.
- A real external-agent protocol rather than a simulated chat.
- Durable local activity and change records.
- Recovery logic that is structurally separate from AI-editable source.

What still needs to be designed:

- Backpack contract and lifecycle.
- Practical Work rail polish for files, screenshots, diffs, previews, and generated artifacts.
- Tool registry and lifecycle.
- Data Source contract.
- The final operator-tool contract for whole-PC work.
- Search and indexing.
- Complete interception and preview coverage for every mutating Hermes tool.
- Secrets and credentials.
- Background operation.
- Packaging and installation of the protected launcher.
- Testing and performance budgets.
- Runtime upgrade compatibility tests.

No future agent should describe the agent as fully working merely because the bridge compiles, Hermes starts, or one model call succeeds. The next proof must perform a creator-approved Windows action, then complete the self-edit preview flow.

### Current machine cleanup required

Earlier broad installer tests installed ffmpeg and Cua outside Papers, then registered the elevated `cua-driver-serve` scheduled task. Papers no longer uses either path. The process was stopped and the creator's User PATH is unchanged. A later check did not find the scheduled task, but future agents should re-check before assuming the machine is clean.

Before the next experience test:

- Re-check whether the `cua-driver-serve` scheduled task exists. If it exists, remove it through one visible, creator-approved UAC prompt.
- Do not delete `%LOCALAPPDATA%\ms-playwright`, `%USERPROFILE%\.cua-driver`, or `%LOCALAPPDATA%\Programs\Cua` automatically; they may be shared. Papers now points Hermes to its own private driver instead.
- Do not automatically uninstall ffmpeg; it may now be shared by other applications.
- Verify no Cua task starts at login.

## The next architectural milestone

Exercise the Agent-first vertical slice with the creator:

1. Re-check the old Cua auto-start task and remove it only if still present, with the creator's visible approval.
2. Confirm Nous Portal remains signed in and the selected free model still works.
3. Verify Pause and Stop interrupt promptly.
4. Enter Inspect mode and request one visible Papers change.
5. Build and experience the temporary version.
6. Reject or keep it based on experience.
7. If kept, verify the canonical `REAL` commit, GitHub push, version activation, and rollback.

Failures found during this exercise should harden the permanent Agent contract before Backpacks are generated.

## Practical agent workbench direction

The immediate interface target is a workbench-style agent UI:

- Conversation in the center.
- Work rail on the side.
- Short public reasoning summaries, never private chain-of-thought.
- Tool/action cards with understandable status.
- First-class file, screenshot, diff, preview, and generated-content artifacts.
- Exact approval cards for consequential actions.
- Self-edit cards that make Build, Experience, Keep, Reject, and Rollback obvious.

This is intentionally narrower than the older Assistant project's larger language around observability, collaboration, and autonomous research. Those ideas remain historical context only. Papers should first become pleasant and reliable for ordinary agent work.

## Decision status language

All future documentation should label statements using one of these statuses:

- **Vision** - desired long-term direction.
- **Confirmed decision** - explicitly accepted by the creator.
- **Current behavior** - implemented and verified.
- **Partial** - some real behavior exists, but the full promise does not.
- **Prototype** - useful for learning, not canonical implementation.
- **Placeholder** - present only to reserve a location or communicate absence.
- **Open question** - not yet decided.
- **Deferred** - intentionally postponed.
- **Superseded** - replaced by a newer decision.
- **Rejected** - intentionally not part of the product.

Never use "implemented," "working," "production-ready," "complete," "safe," "fast," or "verified" without explaining the evidence.

## Source-of-truth hierarchy

For product intent:

1. The creator's latest explicit instruction.
2. Confirmed decisions in this document.
3. Current behavior references explicitly brought into the active discussion.

For implementation reality:

1. Behavior observed in the canonical `REAL` repository.
2. Automated test and build evidence.
3. Current source code.
4. This document.

If the code and documentation disagree, report the disagreement and correct the documentation or implementation deliberately. Do not silently choose the more flattering version.

## Documentation rules

### Keep one canonical context

This file is the primary living context. Do not create competing "final," "latest," or "new-new" context files.

Supporting documents may exist for focused topics, but they must link back here and state their scope.

### Keep obsolete material out of normal handoffs

Do not catalogue obsolete experiments, abandoned files, discarded prototypes, or external local folders that are not part of the canonical Git repository.

When history is genuinely needed to understand a current decision:

- Summarize the lesson.
- Explain how it shaped the present product.
- Omit irrelevant filenames, paths, inventories, and implementation details.
- Do not encourage future agents to inspect obsolete material unless the creator explicitly asks.

The purpose of history is orientation, not archaeology.

### Update documentation with meaningful changes

Update this file when:

- A product law changes.
- A core term changes meaning.
- A feature becomes real.
- A placeholder is replaced.
- A major architectural decision is confirmed.
- A reference prototype is superseded.
- The canonical repository or workflow changes.
- A handoff would otherwise give the next agent an inaccurate picture.

Do not rewrite the entire history for small visual changes.

### Separate intention from evidence

Every progress update should clearly distinguish:

- What the creator wants.
- What was changed.
- What was actually tested.
- What remains unbuilt.
- What assumption was made.

### Use plain language first

The creator should be able to understand every project-status document without learning development jargon.

Technical detail may be included for future agents, but it should follow a plain-language explanation.

### Preserve uncertainty

If the desired behavior is unclear, record the open question. Do not turn a guess into a permanent design.

If a reasonable temporary assumption is needed for a reversible prototype, label it as an assumption.

### Do not document fantasies as systems

Avoid diagrams or descriptions of elaborate subsystems that do not yet exist unless they are clearly labeled Vision or Proposal.

Prefer the smallest useful architecture that supports the next confirmed behavior.

### Do not expose sensitive data

Never commit:

- API keys.
- Login tokens.
- Private vault contents.
- Personal documents.
- Machine credentials.
- Unredacted private paths when a generic description is enough.

The project paths in this document are included intentionally for local handoff and contain no credentials.

### Keep Git history meaningful

The canonical working copy is `REAL`.

Future edits, commits, pulls, and pushes should originate there unless the creator explicitly establishes a new canonical location.

Commit messages should describe the user-visible or architectural outcome, not merely list files.

Do not commit compiled output, secrets, temporary screenshots, or personal vault data.

## Required handoff format

Every substantial handoff should include the following sections.

### Objective

What the creator asked for, in plain language.

### Product meaning

Why the change matters to the intended experience.

### Confirmed decisions

Only decisions explicitly confirmed during the work.

### Assumptions

Any temporary interpretation the agent made.

### Current result

What now exists in the canonical application.

### Verification

Exactly what was built, launched, tested, measured, or visually inspected.

### Not implemented

Anything that might look present but is still a placeholder, mock, partial connection, or future idea.

### Files and commits

The relevant files, commit identifier, branch, and push status.

### Risks or contradictions

Anything that could cause rework, loss of trust, security problems, or architectural conflict.

### Next product question

The smallest question whose answer unlocks the next meaningful step.

## Rules for future agents

1. Read this document before acting.
2. Treat `REAL` as the canonical repository.
3. Do not inspect, modify, or document obsolete external artifacts unless explicitly asked.
4. Treat the older Experience concept only as summarized origin context; the current Backpack model wins.
5. Ask about desired behavior in plain language.
6. Make technical choices on the creator's behalf when they are reversible and clearly within scope.
7. Explain only the tradeoffs that materially affect the creator.
8. Build vertical slices that can be experienced, not broad collections of disconnected stubs.
9. Keep the permanent shell stable while Backpack behavior evolves.
10. Do not make Backpacks into information silos.
11. Do not scope the global AI to one Backpack by accident.
12. Reuse mature agent harnesses and pipelines where appropriate.
13. Prefer visible progress and cancellation for long operations.
14. Protect real files and point back to their locations.
15. Verify performance claims with measurements.
16. Verify experience claims with the creator's testing.
17. Never call a mock response an AI integration.
18. Never call a menu entry a working system.
19. Never call a successful debug build production-ready.
20. Update this document when the truth materially changes.

## Current open questions

- What is the exact Backpack contract?
- What is the exact Tool contract?
- What counts as a Data Source?
- Which Nous model gives the best balance of Windows control, judgment, speed, and usage cost?
- Which Hermes actions do not emit enough information for Papers' exact-preview promise and therefore must remain disabled or wrapped?
- Should the companion start automatically with Windows after the first successful end-to-end test?
- How minimal should Basic remain?
- What measurable performance targets define "lightweight"?

## Decision log

### 2026-07-06 - Clean restart around the clarified vision

The initial proof of concept made the idea tangible but blurred visual simulation with real systems, so it was not adopted as the production foundation.

The creator clarified that Papers is broader than an AI-generated Experience host. It is intended as a universal personal layer over the existing machine.

Backpacks were clarified as overlapping rooms or ways of interacting with shared things. Tools were clarified as reusable machine capabilities. The AI was clarified as global rather than Backpack-limited.

### 2026-07-06 - First Backpack chosen

The first intended Backpack was confirmed as a knowledge-management, second-brain, or Life OS Backpack. The architecture-work Backpack remains an example only.

### 2026-07-06 - Agent-first priority supersedes Backpack-first implementation

The creator decided that manually implementing a first Backpack is no longer the immediate goal. Papers must first gain a solid AI operator and inline editor capable of creating future Backpacks from desired behavior.

Hermes Agent was chosen as the engine behind Papers' own interface. Hermes WebUI remains a reference only.

Confirmed experience decisions:

- Global Ask plus element selection.
- Whole-PC operator visibility under the creator's normal Windows account.
- Full visual Windows control through Hermes Computer Use.
- Exact previews before consequential actions.
- A compact live companion with Pause, Stop, Expand, and inline Ask.
- Staged self-edits with a temporary build, Keep, Revise, Reject, GitHub synchronization, and protected rollback.
- A Papers-managed, pinned Hermes runtime using Nous Portal sign-in.

### 2026-07-06 - Agent-first vertical slice implemented

Papers now contains the real native and interface foundations for the confirmed Agent-first direction. The bridge, local records, policy checks, Inspect metadata, staged Git services, restricted builder MCP, and recovery launcher compile and have local automated coverage.

The external milestone remains partial until Hermes is installed, signed in, and exercised end to end with the creator.

### 2026-07-06 - Native empty desktop foundation created

A clean Tauri desktop repository was created. It compiles and launches as a Windows application and truthfully shows that no Backpack, Tools, Settings, or AI connection exist yet.

### 2026-07-06 - Canonical repository established

`REAL` became the canonical local working repository.

The private GitHub repository `Futahua/papers-are-papers` was created and synchronized on branch `main`.

### 2026-07-07 - Managed runtime exercised and hardened

Papers installed the pinned Hermes Agent 0.18.0 runtime and reached a healthy loopback server. The exercise exposed and fixed an incorrect installer hash, Windows line-ending failures in the upstream fallback path, frontend/native Tauri version drift, vulnerable development dependencies, and an unreachable direct browser WebSocket.

Computer Use is now pinned to Cua Driver 0.7.0, verified against the publisher's SHA-256, and extracted privately without PATH changes or auto-start. Hermes gateway traffic now crosses a native Tauri bridge so the per-launch token remains in Rust memory.

The latest release build and automated checks pass. The release launches and reaches a Hermes ready signal. Nous sign-in is complete in Papers' actual private Hermes home at `%LOCALAPPDATA%\Papers\data\hermes`. The first paid/default model choice failed because the account had insufficient credits, so Papers was switched to the free Nous model `stepfun/step-3.7-flash:free`. A tiny model turn through Papers returned `PAPERS_FIXED_OK`.

During the first app-bridge proof, Hermes emitted reasoning/thinking events and a `reasoning` field inside `message.complete`; Papers initially stored them in SQLite. This violated the rule that Papers must not reveal or store private chain-of-thought. The native storage layer now drops private reasoning/thinking event types and recursively removes private reasoning fields before persistence. The already-captured local database records were scrubbed. Tests verify this behavior.

A harmless inspect-only Computer Use pass against Notepad succeeded and did not edit anything. Notepad restored existing creator tabs rather than a blank scratch file, so the mutation test used an explicitly prepared disposable scratch file in `%TEMP%`. Hermes selected the scratch tab, typed `PAPERS_TYPED_OK`, and did not save; a visual screenshot confirmed the marker was present while the disk file remained unchanged. The unintended elevated Cua scheduled task created by the superseded installer path was not present on the latest check, but future agents should re-check before any real experience test.

### 2026-07-07 - Practical workbench UI direction confirmed

After reviewing the creator's older Assistant project and several agent references, the creator clarified that the near-term need is not observability, agent collaboration, or broad research automation. The desired near-term feel is a practical Codex/Claude-Code-like workbench: clear streaming, safe public reasoning summaries, files/artifacts, diffs, approvals, and self-edit previews. Papers now treats the right-side rail as a Work rail and begins separating chat from work items.
