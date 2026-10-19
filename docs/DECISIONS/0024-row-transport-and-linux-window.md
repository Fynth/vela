# ADR0024 — Explicit row transport and Linux window policy

Accepted2026-10-19; baseline91867f2. No schema/API/dependency change.

A track row represents a transport action when it targets the active occurrence.
Repeated activation pauses/resumes that source without queue reset, source reload,
seek reset or new listening session. Another duplicate occurrence is an explicit new
selection. Context-level Play actions start their context through a separate root
callback. UI names/icons observe requested transport state, not an unrelated local
boolean or only a delayed decoder snapshot.

Native requested play intent survives bounded preparation independently of decoder
acknowledgements. Completion rechecks the existing session/source generation and
reads current intent before loading. A user pause must win over a late completion.
Actual EOF/errors clear intent. Remote and offline transport belongs to the root
coordinator, independent of the currently visible page; playlist metadata still
grants no media permission. Existing caches, integrity checks and cancellation gates
are retained.

Mode availability follows the active source adapter: public shuffle/repeat are
unavailable; offline is sequential with repeat-one and direct local-descriptor EOF
replay. UI and MPRIS share this policy. Owned online mode preferences survive source
changes and are reapplied when returning to the owned online queue.

Production and QA share one window builder. Linux disables the GTK system frame;
window actions fit in the existing account toolbar and drag region. Other platforms
retain their system frame. Verification separates normal tiled Wayland from floated
X11 responsive fixtures, and null-sink playback from physical output.

See [interaction review](../UX_INTERACTION_REVIEW_2026-10-19.md) for actual defects,
identity limits, executed scenario definitions, upstream references and remaining
work. No product-wide or all-platform completeness follows from this increment.
