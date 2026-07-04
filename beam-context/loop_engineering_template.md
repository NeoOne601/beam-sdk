# Loop Engineering 101

Fable is built for fully autonomous work. This document outlines how to design loops once so the AI handles the back-and-forth and only flags you when the job is done.

## The Barbell Strategy for Token Efficiency

- **First 10% (Planning):** Fable 5
- **Middle 80% (Gruntwork):** Subagents like Sonnet, Haiku, and Opus
- **Last 10% (Verification):** Switch back to Fable 5 to verify the final spec.

## Core Commands

- `/goal`: Launches tasks that run until completed. 
  *(Example: "/goal keep researching until you can answer these 5 questions.")*
- `/loop`: Runs intervals that don't stop until you cancel them. 
  *(Example: "/loop every 30 minutes, flag any email that actually needs me.")*

## Skills

A "Skill" is a recipe you teach Claude once, and it reuses forever. Fable 5 proactively tests its own work and is best for iterating on these. Your skills live in your local folder, meaning you own them and can port them across models.

**3 Ways to Build Skills:**
1. **From a Past Chat:** Open an old chat where you did great work, type `/skill creator`, and extract patterns.
2. **From Scratch:** Open Claude Code, hit `skill creator`, and build it based on daily tasks.
3. **From Data:** Pull high-performing examples, feed them into a skill, and tell Claude to mimic tone, structure, and thought patterns.

## Vision Capabilities

Fable 5's most underrated feature. How to use it:
- **Document & Data:** Extract exact data points from complex charts and scientific PDFs.
- **Design & UI:** Drop a screenshot of any user interface for a detailed critique and improvement plan.
- **Development:** Screenshot a dashboard to reverse-engineer the underlying logic.
- **Content & Creative:** Drop reference screenshots to replicate creative direction.
- **Computer Use:** Let Fable read live interfaces and take action without needing helper tools.
