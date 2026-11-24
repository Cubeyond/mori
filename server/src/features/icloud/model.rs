// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Browser controls and redacted synchronization status.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Shared upload optimization preset.
#[derive(Clone, Copy, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Optimization {
    /// Upload unchanged source bytes.
    Original,
    /// Reduce file size with a moderate quality setting.
    #[default]
    Balanced,
    /// Preserve more detail when encoding.
    Quality,
}

impl Optimization {
    /// Store a preset under the same spelling as the wire contract.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Balanced => "balanced",
            Self::Quality => "quality",
        }
    }

    /// Read one of the validated database values.
    pub(super) fn read(value: &str) -> Self {
        match value {
            "original" => Self::Original,
            "quality" => Self::Quality,
            _ => Self::Balanced,
        }
    }
}

/// Shared preference, absent until an existing browser preference is saved.
#[derive(Serialize, ToSchema)]
pub struct UploadPreference {
    /// The account's selected default.
    #[schema(required = true)]
    pub optimization: Option<Optimization>,
}

/// Update the default for subsequent manual uploads and inherited imports.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreferenceInput {
    /// The new default preset.
    pub optimization: Optimization,
}

/// Supported automatic synchronization intervals.
#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    /// Every hour.
    Hourly,
    /// Every six hours.
    SixHours,
    /// Every day at the chosen local time.
    Daily,
    /// Every week on the chosen weekday at the chosen local time.
    Weekly,
}

impl Frequency {
    /// Database representation of the schedule.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::SixHours => "six_hours",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }

    /// Read one of the validated database values.
    pub(super) fn read(value: &str) -> Self {
        match value {
            "hourly" => Self::Hourly,
            "six_hours" => Self::SixHours,
            "weekly" => Self::Weekly,
            _ => Self::Daily,
        }
    }
}

/// Local weekday for a weekly calendar rule.
#[derive(Clone, Copy, Deserialize, Serialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

/// Schedule and the optional per-integration optimization override.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ScheduleInput {
    /// Start background jobs at the specified frequency.
    pub automatic: bool,
    /// How often to check selected albums.
    pub frequency: Frequency,
    /// Day of the week, required for a weekly rule.
    pub weekday: Option<Weekday>,
    /// Local calendar time in 24-hour HH:MM format.
    pub local_time: String,
    /// IANA time zone for calendar scheduling.
    pub timezone: String,
    /// Null inherits the account's upload preference.
    #[schema(required = true)]
    pub optimization: Option<Optimization>,
}

/// Persisted sync policy and last-run summary.
#[derive(FromRow)]
pub(super) struct Schedule {
    /// Whether automatic runs are enabled.
    pub automatic: bool,
    /// Interval identifier.
    pub frequency: String,
    /// Local weekday to use for weekly runs.
    pub weekday: Weekday,
    /// Wall-clock time for calendar runs.
    pub local_time: String,
    /// The chosen IANA timezone.
    pub timezone: String,
    /// An optional override of the shared optimization preference.
    pub optimization: Option<String>,
    /// Next automatic due time or resource retry time.
    pub next_sync: Option<i64>,
    /// Durable due time for a requested or interrupted run.
    pub queued_at: Option<i64>,
    /// Consecutive failures in the currently queued run.
    pub retries: i64,
    /// Most recent completed check.
    pub last_sync: Option<i64>,
    /// Sanitized import feedback.
    pub last_error: Option<String>,
    /// Items imported in the most recent run.
    pub imported: i64,
    /// Items already present or skipped in the most recent run.
    pub skipped: i64,
    /// Retryable failures from the most recent run.
    pub failed: i64,
}

impl Schedule {
    /// Report the persisted schedule under the browser contract.
    pub(super) fn settings(&self) -> ScheduleInput {
        ScheduleInput {
            automatic: self.automatic,
            frequency: Frequency::read(&self.frequency),
            weekday: Some(self.weekday),
            local_time: self.local_time.clone(),
            timezone: self.timezone.clone(),
            optimization: self.optimization.as_deref().map(Optimization::read),
        }
    }
}

/// One source album assigned to an existing Mori album.
#[derive(Clone, Deserialize, FromRow, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AlbumRule {
    /// Complete path returned by iCloud discovery.
    pub source: String,
    /// Existing Mori album identifier.
    pub album_id: String,
}

/// Replace the selected albums in one transaction.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AlbumSelection {
    /// Selected sources and their existing destinations.
    pub albums: Vec<AlbumRule>,
}

/// A discoverable iCloud album path and display name.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct SourceAlbum {
    /// Complete provider path to persist with a mapping.
    pub path: String,
    /// Display label for the source.
    pub name: String,
}

/// Start an Apple login or answer the current verification challenge.
#[derive(Deserialize, ToSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConnectInput {
    /// Use an Apple account password once to start verification.
    Start {
        /// Apple account email.
        account: String,
        /// Apple account password, held only during verification.
        password: String,
        /// Existing browser upload preference to migrate on first use.
        upload_optimization: Optimization,
    },
    /// Answer with a verification code or request SMS.
    Answer {
        /// Opaque challenge identifier.
        challenge: String,
        /// Code, SMS request, or trusted phone choice.
        answer: String,
    },
    /// Discard a pending challenge.
    Cancel {
        /// Opaque challenge identifier.
        challenge: String,
    },
}

/// One trusted phone option, identified by the worker's opaque value.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct PhoneChoice {
    /// Opaque provider choice.
    pub value: String,
    /// Masked trusted phone description.
    pub label: String,
}

/// Verification states displayed by Mori.
#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeKind {
    /// Ask for the code from a trusted device.
    Code,
    /// Ask which trusted phone should receive an SMS.
    Phone,
    /// Ask for the code sent by SMS.
    Sms,
}

/// Public authentication response, with private session material stripped.
#[derive(Serialize, ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConnectResult {
    /// A second step is required.
    Challenge {
        /// Opaque challenge identifier.
        id: String,
        /// Verification step to render.
        kind: ChallengeKind,
        /// Masked choices for the phone selection step.
        phones: Vec<PhoneChoice>,
        /// Whether the provider rejected the preceding answer.
        retry: bool,
    },
    /// A session is available to discover albums.
    Connected,
    /// The pending request was canceled.
    Canceled,
}

/// Current activity of the persisted synchronization queue.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SyncActivity {
    /// Ready for a manual or scheduled run.
    Idle,
    /// Waiting for a worker or available resources.
    Queued,
    /// Actively scanning or importing.
    Running,
}

/// Status visible only to the signed-in Mori administrator.
#[derive(Serialize, ToSchema)]
pub struct SyncStatus {
    /// Whether this deployment enables the optional connector.
    pub enabled: bool,
    /// Connected Apple account email, when available.
    #[schema(required = true)]
    pub account: Option<String>,
    /// Whether the Apple session needs user attention.
    pub needs_auth: bool,
    /// Current activity of the import queue.
    pub activity: SyncActivity,
    /// Current periodic schedule.
    pub schedule: ScheduleInput,
    /// Current default optimization when inheriting the account preference.
    pub upload_optimization: Optimization,
    /// Selected mappings.
    pub albums: Vec<AlbumRule>,
    /// Next automatic run, if enabled.
    #[schema(required = true)]
    pub next_sync: Option<i64>,
    /// Last completed run.
    #[schema(required = true)]
    pub last_sync: Option<i64>,
    /// Sanitized last-run or authentication message.
    #[schema(required = true)]
    pub last_error: Option<String>,
    /// Number of newly imported items in the last run.
    pub imported: i64,
    /// Number of skipped items in the last run.
    pub skipped: i64,
    /// Number of failed items in the last run.
    pub failed: i64,
}
