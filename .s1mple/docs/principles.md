# Principles

These principles drive every decision in this project: discussion, planning, implementation, verification, delivery, and documents. Project and language documents hold the detailed rules. If a rule conflicts with a principle, raise the conflict before proceeding.

## Table of Contents

- [P1. Stable and Simple](#p1-stable-and-simple)
- [P2. Ship First](#p2-ship-first)
- [P3. Risk-Based Testing](#p3-risk-based-testing)
- [P4. Concise, Correct Documents](#p4-concise-correct-documents)

## P1. Stable and Simple

Choose options that are stable, simple, and easy to maintain and extend. Follow project rules and industry standards, and deviate only for a stated reason. Use abstractions when they have a clear purpose.

## P2. Ship First

Shipping is the goal. Keep planning, implementation, verification, and deployment fast, and make the process work for both humans and agents. Deliver small, complete, usable increments, with only as much process as the risk requires.

## P3. Risk-Based Testing

Testing protects shipping speed instead of slowing it. Focus on high-risk behavior, important paths, edge cases, and critical journeys, and avoid trivial or duplicate tests. Cover every level (unit, contract, integration, end-to-end, release), and test each risk at the lowest level that catches it.

## P4. Concise, Correct Documents

Keep documents concise and correct, and follow the [communication rules](communication.md). Keep one source of truth for each fact, and update documents along with the work they describe.
