// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Public HTTP and storage contracts exercise the same interfaces as application callers.
#![expect(
    clippy::unwrap_used,
    reason = "Fixture setup and assertions must fail immediately on invalid test state."
)]

/// Independent behavior contracts grouped by responsibility.
mod contracts;
/// Isolated common fixtures; production never depends on these adapters.
mod support;
