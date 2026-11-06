// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Focused integration contracts share fixtures but never mutable test state.

/// Bounded administrator covers and optional album descriptions.
mod album_catalog;
/// Albums behavior.
mod albums;
/// Auth behavior.
mod auth;
/// Bootstrap behavior.
mod bootstrap;
/// Authentication and storage lifecycle concurrency contracts.
mod concurrency;
/// Delivery behavior.
mod delivery;
/// Shared image/video lifecycle and original-byte preservation.
mod images;
/// Migrations behavior.
mod migrations;
/// Restart-safe object relocation and legacy layout upgrades.
mod objects;
/// Openapi behavior.
mod openapi;
/// Privacy behavior.
mod privacy;
/// Security behavior.
mod security;
/// Storage behavior.
mod storage;
/// Uploads behavior.
mod uploads;

/// Discoverable and idempotent upload cancellation.
mod cancellation;

/// Atomic selected-media membership and deletion.
mod batch;

/// Search pagination and count privacy.
mod search;

/// Anonymous access to the terms accompanying the browser distribution.
mod licensing;
