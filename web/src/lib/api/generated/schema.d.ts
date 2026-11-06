export interface paths {
    "/v1/admin/albums": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List active albums for the administrator, including albums with no public videos.
         * @description # Errors
         *     Returns authentication or database errors.
         */
        get: operations["get_v1_admin_albums"];
        put?: never;
        /**
         * Create an album while atomically reserving its normalized slug.
         * @description # Errors
         *     Returns authentication, CSRF, invalid label/slug, duplicate-slug, or database errors.
         */
        post: operations["post_v1_admin_albums"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/albums/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return one administrator-visible collection, including private members and empty albums.
         * @description # Errors
         *     Returns authentication, missing album, pagination, or database errors.
         */
        get: operations["get_v1_admin_albums_id"];
        put?: never;
        post?: never;
        /**
         * Delete a confirmed album, retaining its videos unless media deletion was explicitly requested.
         * @description # Errors
         *     Returns authentication, confirmation, active media-operation, or database errors.
         */
        delete: operations["delete_v1_admin_albums_id"];
        options?: never;
        head?: never;
        /**
         * Update album label and page visibility without changing member-video access policies.
         * @description # Errors
         *     Returns authentication, CSRF, invalid label/visibility, missing album, or database errors.
         */
        patch: operations["patch_v1_admin_albums_id"];
        trace?: never;
    };
    "/v1/admin/jobs/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return authenticated progress for a durable upload or background job.
         * @description # Errors
         *     Returns a database error while preserving the persisted operation state.
         */
        get: operations["get_v1_admin_jobs_id"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/storage": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return only non-secret configured fields and credential-presence indicators.
         * @description # Errors
         *     Returns authentication or metadata errors.
         */
        get: operations["get_v1_admin_storage"];
        /**
         * Verify and persist settings before atomically activating the new client.
         * @description # Errors
         *     Returns authentication, invalid fields, unsafe target change, connection, encryption, or database errors.
         */
        put: operations["put_v1_admin_storage"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/storage/test": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Check the candidate against S3 without persisting or activating it.
         * @description # Errors
         *     Returns authentication, invalid fields, or failed object-permission checks.
         */
        post: operations["post_v1_admin_storage_test"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/upload-policy": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return environment-configured limits for the authenticated upload interface.
         * @description # Errors
         *     Returns authentication or database errors.
         */
        get: operations["get_v1_admin_upload_policy"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/uploads": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Discover reservations independently of optional local resume fingerprints.
         * @description # Errors
         *     Returns authentication or database failures.
         */
        get: operations["get_v1_admin_uploads"];
        put?: never;
        /**
         * Create a private multipart session with independent random name and URL token.
         * @description # Errors
         *     Returns authentication, CSRF, upload-limit, membership, delivery, S3, or database errors.
         */
        post: operations["post_v1_admin_uploads"];
        /**
         * Cancel an explicit queue snapshot, retaining successful cancellations if storage requires retry.
         * @description # Errors
         *     Returns authentication, CSRF, invalid selection, verification conflicts, or storage failures.
         */
        delete: operations["delete_v1_admin_uploads"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/uploads/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return authenticated progress for a durable upload or background job.
         * @description # Errors
         *     Returns authentication, expired/nonresumable-session, S3, or database errors.
         */
        get: operations["get_v1_admin_uploads_id"];
        put?: never;
        post?: never;
        /**
         * Cancel one original; retrying a cancelled or interrupted cancellation remains safe.
         * @description # Errors
         *     Returns authentication, CSRF, missing reservation, verification conflicts, or storage failures.
         */
        delete: operations["delete_v1_admin_uploads_id"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/uploads/{id}/complete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Validate multipart state and schedule verification or complete the S3 object.
         * @description # Errors
         *     Returns authentication, CSRF, expired-session, state-conflict, or database errors.
         */
        post: operations["post_v1_admin_uploads_id_complete"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/uploads/{id}/parts/{part}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Issue a short-lived signature for one checksummed multipart part.
         * @description # Errors
         *     Returns authentication, CSRF, checksum, part-number, expired-session, S3, or database errors.
         */
        post: operations["post_v1_admin_uploads_id_parts_part"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/videos": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return a bounded collection visible to the authenticated administrator.
         * @description # Errors
         *     Returns authentication, page-bound conversion, or database errors.
         */
        get: operations["get_v1_admin_videos"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/videos/batch": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Apply exactly one atomic operation to a bounded, authorized selection.
         * @description # Errors
         *     Returns authentication, CSRF, invalid selection/album, missing records, or state conflicts.
         */
        post: operations["post_v1_admin_videos_batch"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/admin/videos/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /**
         * Schedule revision-guarded deletion and confirmed withdrawal of any public copies.
         * @description # Errors
         *     Returns authentication, CSRF, state/revision conflict, delivery-configuration, or database
         *     errors.
         */
        delete: operations["delete_v1_admin_videos_id"];
        options?: never;
        head?: never;
        /**
         * Apply metadata changes and atomically schedule visibility transitions.
         * @description # Errors
         *     Returns authentication, CSRF, metadata, membership, delivery-configuration, or revision errors.
         */
        patch: operations["patch_v1_admin_videos_id"];
        trace?: never;
    };
    "/v1/admin/videos/{id}/content": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Serve authenticated private video bytes with HEAD and Range support.
         * @description # Errors
         *     Returns authentication, inaccessible-video, concurrency-limit, S3, or response-building errors.
         */
        get: operations["get_v1_admin_videos_id_content"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        /**
         * Serve authenticated private video bytes with HEAD and Range support.
         * @description # Errors
         *     Returns authentication, inaccessible-video, concurrency-limit, S3, or response-building errors.
         */
        head: operations["head_v1_admin_videos_id_content"];
        patch?: never;
        trace?: never;
    };
    "/v1/admin/videos/{id}/poster": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Serve or store a bounded authenticated JPEG poster.
         * @description # Errors
         *     Returns authentication, inaccessible-poster, concurrency-limit, S3, or response-building errors.
         */
        get: operations["get_v1_admin_videos_id_poster"];
        /**
         * Serve or store a bounded authenticated JPEG poster.
         * @description # Errors
         *     Returns authentication, CSRF, state-conflict, JPEG-limit, S3, or database errors.
         */
        put: operations["put_v1_admin_videos_id_poster"];
        post?: never;
        delete?: never;
        options?: never;
        /**
         * Serve or store a bounded authenticated JPEG poster.
         * @description # Errors
         *     Returns authentication, inaccessible-poster, concurrency-limit, S3, or response-building errors.
         */
        head: operations["head_v1_admin_videos_id_poster"];
        patch?: never;
        trace?: never;
    };
    "/v1/albums/{slug}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return album members authorized by the current administrator session or public visibility.
         * @description # Errors
         *     Returns 404 for inaccessible or missing albums, or an authentication/database error.
         */
        get: operations["get_v1_albums_slug"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Verify the administrator password and establish a revocable session.
         * @description # Errors
         *     Returns authentication, rate-limit, bootstrap, header, password-worker, or database errors.
         */
        post: operations["post_v1_auth_login"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Revoke the current session and expire its browser cookie.
         * @description # Errors
         *     Returns authentication, Origin, CSRF, cookie-header, or database errors.
         */
        post: operations["post_v1_auth_logout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/auth/session": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Report session validity and its associated CSRF token.
         * @description # Errors
         *     Returns a database error; absent or expired sessions produce an unauthenticated response.
         */
        get: operations["get_v1_auth_session"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/source": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Offer the corresponding source without requiring an administrator account. */
        get: operations["get_v1_source"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/v1/videos/{token}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Return only published public content, hiding private and missing targets alike.
         * @description # Errors
         *     Returns 404 for invalid, private, unpublished, or missing tokens, or a database error.
         */
        get: operations["get_v1_videos_token"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        /** @description Mutation acknowledgement with an optional durable background job. */
        Accepted: {
            /** @description Optional durable task identifier for an asynchronous transition. */
            job_id: string | null;
        };
        /** @description Explicitly created album with an editable URL slug and display label. */
        Album: {
            /**
             * Format: int64
             * @description Creation timestamp in Unix seconds.
             */
            created_at: number;
            /** @description Optional administrator-written description; empty values render no placeholder. */
            description: string;
            /** @description Stable internal identifier for this record. */
            id: string;
            /** @description Administrator-chosen album display label, independent of its slug. */
            name: string;
            /** @description Unique editable album URL component. */
            slug: string;
            /** @description Whether the album page can be visited anonymously, independent of member videos. */
            visibility: string;
        };
        /** @description Durable video deletions queued by an explicitly destructive album removal. */
        AlbumDeletion: {
            /** @description Job identifiers whose completion confirms source and public-copy removal. */
            job_ids: string[];
        };
        /** @description Album metadata and a bounded page of videos filtered for the caller. */
        AlbumPage: {
            /** @description Album metadata visible to the caller. */
            album: components["schemas"]["Album"];
            /** @description Whether another result page is available. */
            has_more: boolean;
            /**
             * Format: int64
             * @description Exact number of members authorized for this caller and display-name filter.
             */
            total: number;
            /** @description Visibility-filtered page of video records. */
            videos: components["schemas"]["Video"][];
        };
        /** @description Minimal completed cover; no filenames, object keys, or direct storage URLs are exposed. */
        AlbumPreview: {
            /** @description Stable identifier used to retain the same preview during metadata refreshes. */
            id: string;
            /** @description Media category determines whether the cover has a video indicator. */
            kind: components["schemas"]["MediaKind"];
            /** @description Same-origin authenticated JPEG endpoint, never a signed S3 address. */
            poster_url: string;
        };
        /** @description Album metadata with a bounded, administrator-only set of completed media covers. */
        AlbumSummary: components["schemas"]["Album"] & {
            /** @description Up to three recent posters; originals and incomplete uploads are never returned. */
            previews: components["schemas"]["AlbumPreview"][];
        };
        /** @description Existing or newly created album membership remains independent of media visibility. */
        BatchAction: {
            /** @description Existing album identifier or explicit standalone membership. */
            album_id?: string | null;
            /** @enum {string} */
            type: "move";
        } | {
            /** @description The normal album creation contract, including a separately validated URL slug. */
            album: components["schemas"]["CreateAlbum"];
            /** @enum {string} */
            type: "create_album";
        } | {
            /** @enum {string} */
            type: "delete";
        };
        /** @description One bounded, explicitly selected set; unknown fields cannot broaden destructive scope. */
        BatchInput: {
            /** @description Membership change, new collection, or permanently confirmed deletion. */
            action: components["schemas"]["BatchAction"];
            /** @description Unique internal media identifiers, capped at one hundred per operation. */
            ids: string[];
        };
        /** @description Successful selection changes, optionally containing jobs and a newly created album. */
        BatchResult: {
            /** @description Number of distinct selected records changed atomically. */
            affected: number;
            album: null | components["schemas"]["Album"];
            /** @description Persisted jobs whose completion confirms permanent object removal. */
            job_ids: string[];
        };
        /** @description A bounded snapshot prevents a cancellation from reaching newly started unrelated sessions. */
        CancelUploads: {
            /** @description Exact multipart reservations explicitly cancelled by the administrator. */
            ids: string[];
        };
        /** @description Successful S3 write/read/delete probe, independent of `CloudFront` setup. */
        ConnectionCheck: {
            /** @description Provider actually checked, so a local fixture cannot be reported as a real AWS connection. */
            mode: components["schemas"]["StorageMode"];
            /** @description The supplied S3 configuration passed the required object-permission checks. */
            verified: boolean;
        };
        /** @description Administrator-supplied album label and separately validated slug. */
        CreateAlbum: {
            /** @description Optional plain-text description, trimmed and limited to 500 characters. */
            description?: string;
            /** @description Administrator-chosen album display label, independent of its slug. */
            name: string;
            /** @description Unique editable album URL component. */
            slug: string;
            /** @description Album-page visibility; omitted values default to private. */
            visibility?: string;
        };
        /** @description Upload parameters; initial display names are always generated by the server. */
        CreateUpload: {
            /** @description Optional explicit album membership; absent means standalone. */
            album_id?: string | null;
            /**
             * Format: double
             * @description Browser-probed duration in seconds; zero means unavailable.
             */
            duration?: number;
            /** @description Container extension, independent of the randomized display name. */
            extension?: string;
            /**
             * Format: int64
             * @description Browser-probed display height; zero means unavailable.
             */
            height?: number;
            /**
             * Format: int64
             * @description Expected or validated object length in bytes.
             */
            size: number;
            /** @description Requested public or private access policy. */
            visibility: string;
            /**
             * Format: int64
             * @description Browser-probed display width; zero means unavailable.
             */
            width?: number;
        };
        /** @description Explicit destructive confirmation and optional deletion of contained media. */
        DeleteAlbum: {
            /** @description Exact current album display name, required for both deletion modes. */
            confirmation: string;
            /** @description Queue deletion of every member; false retains and detaches the videos. */
            delete_media?: boolean;
        };
        /**
         * @description Whether changing the optional CDN is safe for the current publication lifecycle.
         * @enum {string}
         */
        DeliveryChange: "available" | "requires_revocation";
        /** @description Editable album label, optional URL slug, and page visibility. */
        EditAlbum: {
            /** @description Omission preserves the description; an empty string clears it. */
            description?: string | null;
            /** @description Administrator-chosen album display label, independent of its slug. */
            name: string;
            /** @description Requested normalized URL slug; omission retains the current address. */
            slug?: string | null;
            /** @description Requested album-page visibility; omission preserves its current value. */
            visibility?: string | null;
        };
        /** @description Editable metadata, optional album membership, and requested visibility. */
        EditVideo: {
            /** @description Optional explicit album membership; absent means standalone. */
            album_id?: string | null;
            /** @description Display name; uploads receive a fresh server-generated hash. */
            name: string;
            /** @description Requested public or private access policy. */
            visibility: string;
        };
        /** @description Persisted storage operation with revision guards, retry state, and CDN progress. */
        Job: {
            /**
             * Format: int64
             * @description Number of failed storage attempts for backoff calculation.
             */
            attempts: number;
            /**
             * Format: int64
             * @description Creation timestamp in Unix seconds.
             */
            created_at: number;
            /** @description Sanitized explanatory message safe to return to the caller. */
            error: string | null;
            /** @description Stable internal identifier for this record. */
            id: string;
            /** @description `CloudFront` invalidation being monitored before confirming withdrawal. */
            invalidation_id: string | null;
            /** @description Persisted publication, verification, revocation, or deletion operation. */
            kind: string;
            /**
             * Format: int64
             * @description Earliest timestamp at which this job may be retried.
             */
            next_at: number;
            /**
             * Format: int64
             * @description Monotonic version guarding obsolete asynchronous operations.
             */
            revision: number;
            /** @description Persisted progress or terminal state of the operation. */
            state: string;
            /** @description Identifier of the associated video record. */
            video_id: string;
        };
        /** @description Password submitted to establish the single administrator session. */
        Login: {
            /** @description Administrator password; never persisted or logged in plaintext. */
            password: string;
            /** @description Configured administrator name; omitted legacy requests use root. */
            username?: string;
        };
        /**
         * @description Media category independent of its container and browser decoding support.
         * @enum {string}
         */
        MediaKind: "image" | "video";
        /** @description Checksum required to authorize a single multipart part. */
        PartInput: {
            /** @description Base64 SHA-256 checksum of a single uploaded part. */
            checksum: string;
        };
        /** @description Temporary S3 upload authorization scoped to one numbered part. */
        PartUrl: {
            /** @description Short-lived presigned URL scoped to one upload part. */
            url: string;
        };
        /** @description Session discovery response, including CSRF only for authenticated callers. */
        Session: {
            /** @description Whether the supplied administrator session is valid. */
            authenticated: boolean;
            /** @description Session-bound CSRF value returned only to its authenticated browser. */
            csrf: string | null;
        };
        /** @description Safe storage summary; credential values are never returned. */
        StorageInfo: {
            /** @description Last four characters identifying the saved access key, never its complete value. */
            access_key_hint: string | null;
            /** @description Existing bucket or an empty value before setup. */
            bucket: string;
            /** @description Whether a validated client is installed. */
            configured: boolean;
            /** @description Public or withdrawing media prevents changing the delivery mode before revocation. */
            delivery_change: components["schemas"]["DeliveryChange"];
            /** @description Optional public-delivery distribution. */
            distribution_id: string | null;
            /** @description Whether a temporary token is stored, without revealing it. */
            has_session_token: boolean;
            /** @description Existing video records prevent moving to an unrelated bucket or region. */
            location_locked: boolean;
            /** @description Provider identity; a test client must never be presented as a real AWS connection. */
            mode: components["schemas"]["StorageMode"];
            /** @description Bucket region, defaulting to the setup example. */
            region: string;
        };
        /** @description Storage fields entered explicitly by the administrator; blank credentials retain saved values. */
        StorageInput: {
            /** @description Replacement key identifier; omission retains the saved credential. */
            access_key_id?: string | null;
            /** @description Private Amazon S3 bucket. */
            bucket: string;
            /** @description Optional distribution; omission selects direct, signed S3 playback. */
            distribution_id?: string | null;
            /** @description AWS region hosting the bucket. */
            region: string;
            /** @description Replacement secret; never included in a response. */
            secret_access_key?: string | null;
            /** @description Replacement temporary token; an empty string clears it and omission retains it. */
            session_token?: string | null;
        };
        /**
         * @description Installed provider, distinguishing local protocol fixtures from Amazon S3.
         * @enum {string}
         */
        StorageMode: "unconfigured" | "amazon_s3" | "test";
        /** @description Non-secret queue metadata remains available after browser storage has been cleared. */
        UnfinishedUpload: {
            /** @description Album whose deletion may be blocked by this upload. */
            album_id: string | null;
            /** @description Multipart reservation identity; no AWS upload credentials are exposed. */
            id: string;
            /** @description Server-assigned display hash for the unfinished original. */
            name: string;
            /**
             * Format: int64
             * @description Expected source length in bytes.
             */
            size: number;
            /** @description Media record associated with this unfinished original. */
            video_id: string;
        };
        /** @description Resumable upload details with the server-assigned display hash. */
        Upload: {
            /**
             * Format: int64
             * @description Absolute expiry timestamp in Unix seconds.
             */
            expires_at: number;
            /** @description Stable internal identifier for this record. */
            id: string;
            /** @description Display name; uploads receive a fresh server-generated hash. */
            name: string;
            /**
             * Format: int32
             * @description Number of parts required by the declared file size.
             */
            part_count: number;
            /**
             * Format: int64
             * @description Planned non-final part size in bytes.
             */
            part_size: number;
            /** @description Verified uploaded parts available for resumable transfer. */
            parts: components["schemas"]["UploadedPart"][];
            /** @description Identifier of the associated video record. */
            video_id: string;
        };
        /** @description Safe upload limits exposed without credentials or storage configuration. */
        UploadPolicy: {
            /** @description Enabled lowercase container extensions. */
            allowed_extensions: string[];
            /** @description Image containers used by clients to choose image metadata and preview generation. */
            image_extensions: string[];
            /**
             * Format: int64
             * @description Independent maximum original-image length in bytes.
             */
            max_image_upload_bytes: number;
            /**
             * Format: int64
             * @description Maximum accepted video length in bytes.
             */
            max_upload_bytes: number;
        };
        /** @description S3-confirmed part metadata used to resume an interrupted upload. */
        UploadedPart: {
            /** @description Base64 SHA-256 checksum of a single uploaded part. */
            checksum: string | null;
            /**
             * Format: int32
             * @description One-based S3 multipart part number.
             */
            number: number;
            /**
             * Format: int64
             * @description Expected or validated object length in bytes.
             */
            size: number;
        };
        /** @description Image/video metadata; the legacy DTO name and API routes preserve existing integrations. */
        Video: {
            /** @description Optional explicit album membership; absent means standalone. */
            album_id: string | null;
            /**
             * Format: int64
             * @description Creation timestamp in Unix seconds.
             */
            created_at: number;
            /**
             * Format: double
             * @description Validated movie duration in seconds.
             */
            duration: number;
            /** @description Immutable original container extension used in public resource URLs. */
            extension: string;
            /**
             * Format: int64
             * @description Displayed video height after track rotation is accounted for.
             */
            height: number;
            /** @description Stable internal identifier for this record. */
            id: string;
            /** @description Original media category, independent of extension or browser codec support. */
            kind: components["schemas"]["MediaKind"];
            /** @description Sanitized actionable failure shown in administrator management. */
            last_error: string | null;
            /** @description Public direct URL or authenticated private content endpoint. */
            media_url: string;
            /** @description Display name; uploads receive a fresh server-generated hash. */
            name: string;
            /**
             * Format: int64
             * @description Whether a JPEG poster has been stored.
             */
            poster: number;
            /** @description Optional public or authenticated poster endpoint. */
            poster_url: string | null;
            /**
             * Format: int64
             * @description Monotonic version guarding obsolete asynchronous operations.
             */
            revision: number;
            /**
             * Format: int64
             * @description Expected or validated object length in bytes.
             */
            size: number;
            /** @description Persisted progress or terminal state of the operation. */
            state: string;
            /** @description Unpredictable public video identifier independent of display names. */
            token: string;
            /** @description Requested public or private access policy. */
            visibility: string;
            /**
             * Format: int64
             * @description Displayed video width after track rotation is accounted for.
             */
            width: number;
        };
        /** @description Bounded video collection with continuation information. */
        VideoPage: {
            /** @description Whether another result page is available. */
            has_more: boolean;
            /**
             * Format: int64
             * @description Exact authorized result count after applying the display-name filter.
             */
            total: number;
            /** @description Visibility-filtered page of video records. */
            videos: components["schemas"]["Video"][];
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    get_v1_admin_albums: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlbumSummary"][];
                };
            };
        };
    };
    post_v1_admin_albums: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateAlbum"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Album"];
                };
            };
            /** @description Slug already used */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_admin_albums_id: {
        parameters: {
            query?: {
                /** @description Optional pagination offset clamped to the supported range. */
                offset?: number;
                /** @description Optional page size clamped to the configured response budget. */
                limit?: number;
                /** @description Optional literal display-name search, limited to 200 characters. */
                q?: string;
            };
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlbumPage"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_v1_admin_albums_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DeleteAlbum"];
            };
        };
        responses: {
            /** @description Album removed; video deletion jobs queued */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlbumDeletion"];
                };
            };
            /** @description Album removed; videos retained without album membership */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Confirmation does not match */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Album not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description A member video has an operation in progress */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    patch_v1_admin_albums_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EditAlbum"];
            };
        };
        responses: {
            /** @description Album updated */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_admin_jobs_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Job"];
                };
            };
        };
    };
    get_v1_admin_storage: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StorageInfo"];
                };
            };
        };
    };
    put_v1_admin_storage: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StorageInput"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["StorageInfo"];
                };
            };
            /** @description Existing media locks the storage location */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    post_v1_admin_storage_test: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["StorageInput"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ConnectionCheck"];
                };
            };
        };
    };
    get_v1_admin_upload_policy: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["UploadPolicy"];
                };
            };
        };
    };
    get_v1_admin_uploads: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["UnfinishedUpload"][];
                };
            };
        };
    };
    post_v1_admin_uploads: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateUpload"];
            };
        };
        responses: {
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Upload"];
                };
            };
        };
    };
    delete_v1_admin_uploads: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CancelUploads"];
            };
        };
        responses: {
            /** @description Uploads cancelled */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_admin_uploads_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Upload"];
                };
            };
        };
    };
    delete_v1_admin_uploads_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Aborted */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    post_v1_admin_uploads_id_complete: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Accepted"];
                };
            };
        };
    };
    post_v1_admin_uploads_id_parts_part: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
                part: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PartInput"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PartUrl"];
                };
            };
        };
    };
    get_v1_admin_videos: {
        parameters: {
            query?: {
                /** @description Optional pagination offset clamped to the supported range. */
                offset?: number;
                /** @description Optional page size clamped to the configured response budget. */
                limit?: number;
                /** @description Optional literal display-name search, limited to 200 characters. */
                q?: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["VideoPage"];
                };
            };
        };
    };
    post_v1_admin_videos_batch: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BatchInput"];
            };
        };
        responses: {
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["BatchResult"];
                };
            };
            /** @description Media changed */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    delete_v1_admin_videos_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Accepted"];
                };
            };
        };
    };
    patch_v1_admin_videos_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EditVideo"];
            };
        };
        responses: {
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Accepted"];
                };
            };
            /** @description Operation in progress */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_admin_videos_id_content: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Authenticated video stream */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Partial video */
            206: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unsatisfiable range */
            416: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    head_v1_admin_videos_id_content: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Authenticated video stream */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Partial video */
            206: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Unsatisfiable range */
            416: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_admin_videos_id_poster: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Authenticated poster */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    put_v1_admin_videos_id_poster: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "image/jpeg": Blob;
            };
        };
        responses: {
            /** @description Poster saved */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    head_v1_admin_videos_id_poster: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Authenticated poster */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_albums_slug: {
        parameters: {
            query?: {
                /** @description Optional pagination offset clamped to the supported range. */
                offset?: number;
                /** @description Optional page size clamped to the configured response budget. */
                limit?: number;
                /** @description Optional literal display-name search, limited to 200 characters. */
                q?: string;
            };
            header?: never;
            path: {
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AlbumPage"];
                };
            };
            /** @description Not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    post_v1_auth_login: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Login"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Session"];
                };
            };
            /** @description Invalid password */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Try later */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    post_v1_auth_logout: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Session revoked */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_auth_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Session"];
                };
            };
        };
    };
    get_v1_source: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Corresponding source for this running version */
            307: {
                headers: {
                    /** @description Public source repository or archive */
                    Location?: string;
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_v1_videos_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                token: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Video"];
                };
            };
            /** @description Not public */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
}
