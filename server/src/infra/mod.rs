// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

//! HTTP and S3 orchestration regression tests using isolated protocol fixtures.

/// SQLite WAL initialization and embedded migrations.
pub mod db;
/// Checksummed S3 operations and bounded AWS clients.
pub mod storage;

/// Encrypted configuration and runtime storage-client management.
pub mod storage_settings;
