// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! Mori application modules for authenticated video management and S3 delivery.

/// Validated configuration and shared resource budgets.
pub mod app;
/// Domain operations for authentication, albums, uploads, and videos.
pub mod features;
/// HTTP routing, OpenAPI generation, and response security policies.
pub mod http;
/// SQLite and AWS adapters with bounded resource use.
pub mod infra;
/// Durable storage transitions, verification, and cleanup.
pub mod jobs;
