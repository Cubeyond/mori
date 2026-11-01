//! Redacted connection-test feedback derived only from SDK error categories and S3 codes.

use crate::http::error::Error;
use aws_sdk_s3::{
    error::{ProvideErrorMetadata, SdkError},
    operation::{
        delete_object::DeleteObjectError, head_object::HeadObjectError, put_object::PutObjectError,
    },
};
use axum::http::StatusCode;

/// Object permission exercised when the connection probe failed.
#[derive(Clone, Copy)]
enum Operation {
    /// Upload the random zero-byte test object.
    Write,
    /// Inspect the test object's metadata.
    Read,
    /// Remove the test object after inspection.
    Delete,
}
impl Operation {
    /// Name the exact required permission without including a bucket, key, or credential.
    const fn denied(self) -> &'static str {
        match self {
            Self::Write => {
                "S3 denied writing the test object (s3:PutObject). Check IAM and bucket policies for private/*."
            }
            Self::Read => {
                "S3 denied reading the test object (s3:GetObject). Check IAM and bucket policies for private/*."
            }
            Self::Delete => {
                "S3 denied deleting the test object (s3:DeleteObject). Check IAM and bucket policies for private/*."
            }
        }
    }
    /// Preserve the failed operation when S3 returns an unrecognized response.
    const fn rejected(self) -> &'static str {
        match self {
            Self::Write => {
                "S3 rejected the test object upload. Check bucket encryption and account restrictions."
            }
            Self::Read => {
                "S3 rejected the test object metadata request. Check bucket permissions and encryption settings."
            }
            Self::Delete => {
                "S3 rejected test object deletion. Check bucket permissions and retention settings."
            }
        }
    }
}

/// Convert a concrete SDK failure into safe, actionable setup feedback.
fn message<E: ProvideErrorMetadata>(error: &SdkError<E>, operation: Operation) -> &'static str {
    match error {
        SdkError::TimeoutError(_) => {
            return "The S3 request timed out. Check the server's network connection to AWS.";
        }
        SdkError::DispatchFailure(_) => {
            return "Cannot connect to Amazon S3 over HTTPS. Check server networking and TLS certificates.";
        }
        SdkError::ConstructionFailure(_) => {
            return "Could not build the S3 request. Check the storage settings.";
        }
        SdkError::ResponseError(_) => return "Could not read a valid response from Amazon S3.",
        _ => {}
    }
    let code = error
        .as_service_error()
        .and_then(ProvideErrorMetadata::code);
    match code {
        Some("InvalidAccessKeyId") => {
            "Amazon S3 does not recognize this Access key ID. Check that the IAM access key is active."
        }
        Some("SignatureDoesNotMatch" | "InvalidSignatureException") => {
            "Amazon S3 rejected the signature. Check the Secret access key and server clock."
        }
        Some("ExpiredToken" | "InvalidToken" | "TokenRefreshRequired" | "InvalidSecurity") => {
            "AWS session credentials are invalid or expired. Replace the temporary credentials and Session token."
        }
        Some("RequestTimeTooSkewed" | "RequestExpired") => {
            "The server clock is out of sync with AWS. Synchronize the server time and retry."
        }
        Some(
            "PermanentRedirect"
            | "AuthorizationHeaderMalformed"
            | "IllegalLocationConstraintException",
        ) => "The configured AWS region does not match the S3 bucket region.",
        Some("NoSuchBucket") => {
            "Amazon S3 could not find this bucket. Enter the complete bucket name, including its account and region suffix."
        }
        Some("AccessDenied" | "Forbidden" | "AllAccessDisabled") => operation.denied(),
        _ => match error
            .raw_response()
            .map(|response| response.status().as_u16())
        {
            Some(403) => operation.denied(),
            Some(301 | 307) => "The configured AWS region does not match the S3 bucket region.",
            Some(404) => {
                "Amazon S3 could not find the test bucket or object. Check the complete bucket name."
            }
            Some(500..=599) => "Amazon S3 is temporarily unavailable. Retry the connection test.",
            _ => operation.rejected(),
        },
    }
}

/// Preserve known probe failures while never formatting SDK requests, responses, or credential-bearing errors.
pub(super) fn connection_error(error: &anyhow::Error) -> Error {
    let reason = match (
        error.downcast_ref::<SdkError<PutObjectError>>(),
        error.downcast_ref::<SdkError<HeadObjectError>>(),
        error.downcast_ref::<SdkError<DeleteObjectError>>(),
    ) {
        (Some(error), _, _) => message(error, Operation::Write),
        (_, Some(error), _) => message(error, Operation::Read),
        (_, _, Some(error)) => message(error, Operation::Delete),
        _ => "S3 connection check failed. Verify the bucket, AWS region and object permissions.",
    };
    Error(StatusCode::BAD_GATEWAY, reason)
}
