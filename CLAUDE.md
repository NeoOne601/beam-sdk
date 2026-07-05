# CLAUDE.md — Ajna Context & Rules

This file provides the primary instruction set and architectural rules for Claude Code (Fable 5) when executing long-horizon tasks in this workspace. 

## Project Identity & Rebrand
This project was previously known as "Ajna". **It has been rebranded to "Ajna".** All new code, architectures, crates, and variables must reflect the Ajna naming convention (e.g., `ajna-core`, `ajna-verify-backend`, `ajna-idv`, `ajna-intel`, `ajna-vision`).

## What this is
Ajna is a complete Go-To-Market (GTM) security platform suite with three pillars:
1. **Ajna IDV:** Document scanning and identity verification.
2. **Ajna Intel:** Device Posture and Intelligence (fraud/jailbreak detection).
3. **Ajna Vision:** Facial Liveness and Face Verification.

The platform relies on native execution with **post-quantum cryptographic (PQC) result integrity** at the edge. The Rust core performs all business logic and PQC signing. C++ handles zero-copy ML tensor delivery. The backend is an Axum service handling verification. 
**New Additions:** 
- An **MCP Server (`ajna-mcp-server`)** makes this platform agentic and easily consumable by other AI agents.
- **60-Minute Onboarding:** A premium dashboard and integration portal makes client setup seamless.
- **Client UI Customization:** The SDK must expose a headless mode and a declarative UI configuration layer. End-user businesses must be able to completely customize the capture UI (colors, overlays, animations, branding) or build their own UI entirely on top of the Ajna core to fit their company's needs.

## Current Architecture & State
Before making any non-trivial changes or starting a `/goal` loop, you **must** read:
- @README.md: This is the primary design document. It contains the full architecture, sequence diagrams, C++ FFI boundaries, and the critical security remediation log (VR-1..VR-6).

## Compliance Constraints (Strict Rules)
- **SOC2 Type 2:** All backend actions and verification results MUST be written to an append-only, tamper-evident audit log in PostgreSQL.
- **Indian National Quantum Mission (NQM):** The `ajna-crypto` crate must align with Indian Quantum Mission standards, ensuring cryptographic agility (e.g., dynamic negotiation of ML-DSA and classical algorithms).
- **Zero Plagiarism:** The architecture must remain entirely unique and cannot plagiarize proprietary architectures (e.g., Ajna.com).

## The Barbell Strategy for Execution (For Claude Code)
When executing `/goal`, you must use the Barbell Strategy:
1. **First 10% (Planning):** You (Fable 5) map out the architectural steps, read `CLAUDE.md` and `memory.md`, and write out the spec. **CRITICAL FIRST CHECK:** Before writing any code, you must review the existing architecture decisions in `README.md` to ensure they maintain blazing fast performance on a budget Helio G85 device, guarantee zero-copy frame delivery across HAL implementations, and gracefully degrade when the ANE (Apple Neural Engine) or hardware accelerators are unavailable. You must also verify the architecture sets Ajna up to be the most secure, highly customizable, and absolute best-in-class product on the market. Furthermore, plan the abstraction layer that allows client businesses to fully customize the SDK's capture UI or run it headlessly.
2. **Middle 80% (Gruntwork):** Delegate the heavy code-writing to subagents. In Claude Code, you can do this by using subagent tools (like Opus, Sonnet or Haiku) to execute boilerplate tasks, scaffold the UI, and write tests, while you remain the orchestrator tracking the termination condition.
3. **Last 10% (Verification):** You (Fable 5) step back in to audit the final codebase against the compliance constraints, run integration tests, and finalize the loop.

## Hardware Constraints (8GB M1 Mac)
The host machine is an M1 iMac with strictly 8GB of RAM. To prevent Out-Of-Memory (OOM) crashes and swap thrashing during autonomous loops, you MUST adhere to the following:
- **Limit Compilation Concurrency:** Never run standard `cargo test` or `cargo build` which spawns many threads. Always restrict jobs using `-j 2` (e.g., `cargo test --release -j 2`).
- **Constrain Docker:** When spinning up `docker-compose` for the backend, ensure Postgres and Redis have strict memory limits applied (`mem_limit: 256m`) so they do not starve the Rust compiler.
- **Sequential Execution:** Do not instruct subagents to run heavy builds in parallel. Orchestrate tasks sequentially.

## Workflow Orchestration

### 1. Plan Mode Default
- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately
- Use plan mode for verification steps, not just building
- Write detailed specs upfront to reduce ambiguity

### 2. Subagent Strategy
- Use subagents liberally to keep main context window clean
- Offload research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One task per subagent for focused execution

### 3. Self-Improvement Loop
- After ANY correction from the user: update `tasks/lessons.md` with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review lessons at session start for relevant project

### 4. Verification Before Done
- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness

### 5. Demand Elegance (Balanced)
- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes -- don't over-engineer
- Challenge your own work before presenting it

### 6. Autonomous Bug Fixing
- When given a bug report: just fix it. Don't ask for hand-holding
- Point at logs, errors, failing tests -- then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how

## Task Management

1. **Plan First:** Write plan to `tasks/todo.md` with checkable items
2. **Verify Plan:** Check in before starting implementation
3. **Track Progress:** Mark items complete as you go
4. **Explain Changes:** High-level summary at each step
5. **Document Results:** Add review section to `tasks/todo.md`
6. **Capture Lessons:** Update `tasks/lessons.md` after corrections

## Core Principles

- **Simplicity First:** Make every change as simple as possible. Impact minimal code.
- **No Laziness:** Find root causes. No temporary fixes. Senior developer standards.
- **Minimal Impact:** Only touch what's necessary. No side effects with new bugs.

## Accumulated Memory
@beam-context/memory.md