# Fable Prompts Templates

Use these templates directly with Claude Code (Fable 5) to automate workflows.

---

## 1. The /goal Loop
**Purpose:** Let Fable run until a task is completely done, verifying its own work.

```text
/goal [WHAT YOU WANT DONE — e.g. "refactor src/api to async/await throughout"]

Success criteria:
- [criterion 1 — e.g. "all API routes use async/await, no callbacks"]
- [criterion 2 — e.g. "all tests pass (npm test)"]
- [criterion 3 — e.g. "no new type errors (npm run type-check)"]

Rules:
- Plan first, then execute in small, verifiable steps
- After each step, run the checks and fix anything that fails
- Only stop when every criterion is met, OR after [N] attempts — then report what's blocking you
- Don't ask me to confirm routine steps; keep going
```

---

## 2. The Interval (/loop) Version
**Purpose:** Run a recurring task on a schedule until cancelled.

```text
/loop [YOUR RECURRING TASK — e.g. "every 30 minutes, scan my inbox and flag only the emails that genuinely need me, with a one-line reason each"]

Keep running on that interval until I cancel. Summarise what you did each cycle.
```

---

## 3. The Skill-Creator Prompt
**Purpose:** Teach Claude once, reuse forever. Turn workflows into reusable skills.

```text
Help me turn a workflow I repeat into a reusable skill.

The workflow: [DESCRIBE WHAT YOU DO OVER AND OVER — the inputs, the steps, and what "good output" looks like to you]

Do this:
1. Interview me with a few sharp questions to capture the steps and standards I keep in my head
2. Draft a clean, self-contained skill file: purpose, when to use it, step-by-step, output format, and hard rules
3. Show me how to save it and trigger it
4. Suggest 2-3 ways to make it more robust over time

Start by asking your questions — one at a time.
```

**Build from data variant:**
```text
Here are [examples of my work / my best posts / my past outputs].

Study them and build a skill that mimics my tone, structure, and thinking. Then, each time you produce something, I'll tell you what landed and what flopped — update the skill from that feedback.

[PASTE 5-20 EXAMPLES HERE]
```

---

## 4. The CLAUDE.md Context Scaffold
**Purpose:** Give Claude a memory of your world. Drop this in `CLAUDE.md` and point Claude Code at it.

```md
# CLAUDE.md — my context

## About me / my business
- What I do: [one line]
- Who's involved: [key people + roles]
- Current priorities: [top 1-3 right now]

## How to behave
- Every time I share major context about my business or situation, log the key details to a running memory file.
- Always reference past decisions before making new recommendations.
- When I ask for strategy, assume you know everything in this folder.
- Format all outputs in markdown unless I say otherwise.
- Always / never: [your hard rules — e.g. "never use emojis", "always give me options, not one answer"]

## What lives in this folder
- Company map (who does what, current priorities)
- SOPs for anything I do repeatedly
- One-pagers for key clients, projects, meetings
- Strategy docs (launch plans, content systems)
- A running log of decisions + outcomes
```
