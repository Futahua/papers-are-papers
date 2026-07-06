# Papers are Papers - Project Context and Handoff Rules

Last updated: 2026-07-06  
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

The global AI is not connected in the current app.

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

## The first intended Backpack

The first Backpack to design is a knowledge-management, second-brain, or Life OS Backpack.

The architecture-work Backpack described in early sketches was an example demonstrating the range of possible Backpacks. It is not the first implementation target.

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

Current commit at the time this document was prepared:

`5c3f396 - Create empty Papers desktop foundation`

The repository currently contains:

- A real Tauri Windows desktop application.
- A minimal Rust-native host process.
- A static HTML/CSS interface embedded inside the desktop window.
- A restrictive starting capability set.
- A deterministic placeholder application icon.
- A Cargo lockfile.
- Generated Tauri schemas.

The presence of HTML and CSS inside Tauri does not make this a website. They currently draw the interface inside a real desktop application. Machine access, background work, and privileged behavior must remain in the native application layer rather than being faked in the interface.

### What has been verified

- The committed source compiled successfully as a Windows Tauri desktop app.
- The debug executable launched.
- The main window title was `PAPERS ARE PAPERS`.
- The process remained responsive during the launch check.
- The repository is private on GitHub.
- The `main` branch is synchronized between `REAL` and GitHub.
- Build output under `src-tauri/target` is ignored by Git.

### What the current app honestly does

- Opens a real desktop window.
- Displays the intentionally empty starting state.
- Shows that there is no Backpack.
- Shows zero Tools.
- Shows Settings as unset.
- Shows that no AI agent is connected.
- Provides a minimal permanent-shell concept for Basic and Ask.

### What the current app does not do

- Create, open, save, or switch Backpacks.
- Connect or run Tools.
- Store Settings.
- Connect to an AI agent.
- Read files or databases.
- Index or search the machine.
- Run in the background.
- Start with Windows.
- Use a system tray.
- Persist application state.
- Manage permissions beyond its minimal starting capability.
- Install itself.
- Update itself.
- Recover from crashes.
- Record history.
- Provide production security.
- Provide production logging or diagnostics.
- Prove startup, memory, indexing, or interaction performance at real scale.

The empty UI controls intentionally do not pretend these systems exist.

## Current architectural assessment

The current app is a sound minimal native seed. It is not a complete or "perfect" architecture for the full vision.

What is directionally correct:

- Windows desktop application.
- Tauri as a lightweight native host.
- Rust available for machine-level responsibilities.
- A flexible interface layer.
- Restrictive initial permissions.
- Independent clean repository.
- Very little existing technical debt.

What still needs to be designed:

- Backpack contract and lifecycle.
- Tool registry and lifecycle.
- Data Source contract.
- Agent adapter contract.
- Shared event and task model.
- Durable local state.
- Search and indexing.
- Permission and approval model.
- Secrets and credentials.
- Background operation.
- Recovery and history.
- Testing and performance budgets.
- Installation and updates.

No future agent should describe the architecture as finished merely because the empty window builds.

## The next architectural milestone

Before implementing the Life OS Backpack, define four small permanent contracts in plain language:

1. Backpack.
2. Tool.
3. Data Source.
4. Agent.

For each contract, establish:

- What it represents to the user.
- What it owns.
- What it may reference.
- What it may never own.
- How it is created.
- How it is found.
- How it changes.
- How it is disabled or removed.
- What survives application restarts.
- How permissions and failure are shown.

Do not build an elaborate framework before these meanings are settled.

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

- What should the Life OS Backpack show immediately when entered?
- What is the smallest real dataset that should demonstrate it?
- What is the exact Backpack contract?
- What is the exact Tool contract?
- What counts as a Data Source?
- Which existing agent harness should Papers connect to first?
- How should the global AI behave when uncertain?
- Which actions require explicit approval?
- What state should persist across restarts?
- What should Papers do while no Backpack exists?
- How minimal should Basic remain?
- When should Papers run in the background?
- What measurable performance targets define "lightweight"?

## Decision log

### 2026-07-06 - Clean restart around the clarified vision

The initial proof of concept made the idea tangible but blurred visual simulation with real systems, so it was not adopted as the production foundation.

The creator clarified that Papers is broader than an AI-generated Experience host. It is intended as a universal personal layer over the existing machine.

Backpacks were clarified as overlapping rooms or ways of interacting with shared things. Tools were clarified as reusable machine capabilities. The AI was clarified as global rather than Backpack-limited.

### 2026-07-06 - First Backpack chosen

The first intended Backpack was confirmed as a knowledge-management, second-brain, or Life OS Backpack. The architecture-work Backpack remains an example only.

### 2026-07-06 - Native empty desktop foundation created

A clean Tauri desktop repository was created. It compiles and launches as a Windows application and truthfully shows that no Backpack, Tools, Settings, or AI connection exist yet.

### 2026-07-06 - Canonical repository established

`REAL` became the canonical local working repository.

The private GitHub repository `Futahua/papers-are-papers` was created and synchronized on branch `main`.
