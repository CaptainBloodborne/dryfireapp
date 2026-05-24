use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::domain::errors::{DomainError, ValidationError};

#[derive(Serialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetails,
}

#[derive(Serialize)]
pub struct ApiErrorDetails {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// HTTP-layer error wrapper. Built from a [`DomainError`] (or any
/// `Into<DomainError>`). Implements `IntoResponse` so handlers can
/// return `Result<T, ApiError>`.
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub request_id: Option<String>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, rid: Option<String>) -> Self {
        self.request_id = rid;
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message)
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;
        let body = Json(ApiErrorBody {
            error: ApiErrorDetails {
                code: self.code,
                message: self.message,
                request_id: self.request_id,
            },
        });
        (status, body).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        // Log full error chain for 5xx; never leak it to the client.
        if matches!(e, DomainError::Infra(_)) {
            tracing::error!(error = ?e, "internal error");
        } else {
            tracing::debug!(error = ?e, "domain error");
        }

        use DomainError::*;
        let (status, code) = match &e {
            EmailAlreadyExists => (StatusCode::CONFLICT, "USER_EMAIL_ALREADY_EXISTS"),
            LoginAlreadyTaken => (StatusCode::CONFLICT, "USER_LOGIN_ALREADY_TAKEN"),
            Underage { .. } => (StatusCode::BAD_REQUEST, "USER_UNDERAGE"),
            UserNotFound => (StatusCode::NOT_FOUND, "USER_NOT_FOUND"),
            NotVerified => (StatusCode::FORBIDDEN, "USER_NOT_VERIFIED"),
            AlreadyVerified => (StatusCode::CONFLICT, "USER_ALREADY_VERIFIED"),
            Blocked => (StatusCode::FORBIDDEN, "USER_BLOCKED"),

            InvalidCredentials => (StatusCode::UNAUTHORIZED, "AUTH_INVALID_CREDENTIALS"),
            InvalidToken => (StatusCode::UNAUTHORIZED, "AUTH_INVALID_TOKEN"),
            TokenExpired => (StatusCode::UNAUTHORIZED, "AUTH_TOKEN_EXPIRED"),
            SessionRevoked => (StatusCode::UNAUTHORIZED, "AUTH_SESSION_REVOKED"),

            InvalidVerificationToken => {
                (StatusCode::BAD_REQUEST, "AUTH_INVALID_VERIFICATION_TOKEN")
            }
            InvalidResetToken => (StatusCode::BAD_REQUEST, "AUTH_INVALID_RESET_TOKEN"),

            Validation(v) => return validation_to_api_error(v),

            Infra(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),

            GunNotFound => (StatusCode::NOT_FOUND, "GUN_NOT_FOUND"),
            GunSerialAlreadyExists => (StatusCode::CONFLICT, "GUN_SERIAL_DUPLICATE"),
            AmmoLotNotFound => (StatusCode::NOT_FOUND, "AMMO_LOT_NOT_FOUND"),
            AmmoInsufficient { .. } => (StatusCode::CONFLICT, "AMMO_INSUFFICIENT"),
            LicenseNotFound => (StatusCode::NOT_FOUND, "LICENSE_NOT_FOUND"),
            LicenseExpired => (StatusCode::CONFLICT, "LICENSE_EXPIRED"),
            BallisticProfileNotFound => (StatusCode::NOT_FOUND, "BALLISTIC_PROFILE_NOT_FOUND"),
            BallisticInput(_) => (StatusCode::BAD_REQUEST, "BALLISTIC_INPUT_INVALID"),
            ScopeProfileNotFound => (StatusCode::NOT_FOUND, "SCOPE_PROFILE_NOT_FOUND"),
            ScopeRangeExceeded => (StatusCode::BAD_REQUEST, "SCOPE_RANGE_EXCEEDED"),
            LawNotFound => (StatusCode::NOT_FOUND, "LAW_NOT_FOUND"),
            NotOwner => (StatusCode::FORBIDDEN, "NOT_OWNER"),
        };

        let message = if status.is_server_error() {
            "internal error".to_string()
        } else {
            e.to_string()
        };
        Self {
            status,
            code,
            message,
            request_id: None,
        }
    }
}

impl From<ValidationError> for ApiError {
    fn from(v: ValidationError) -> Self {
        // You can pass it by reference since your helper expects &ValidationError
        validation_to_api_error(&v)
    }
}

fn validation_to_api_error(v: &ValidationError) -> ApiError {
    use ValidationError::*;
    let code: &'static str = match v {
        Login => "VALIDATION_LOGIN",
        Email => "VALIDATION_EMAIL",
        PasswordTooShort => "VALIDATION_PASSWORD_TOO_SHORT",
        PasswordTooLong => "VALIDATION_PASSWORD_TOO_LONG",
        PasswordNoDigit => "VALIDATION_PASSWORD_NO_DIGIT",
        PasswordNoUppercase => "VALIDATION_PASSWORD_NO_UPPERCASE",
        PasswordNoLowercase => "VALIDATION_PASSWORD_NO_LOWERCASE",
        PasswordNoSpecial => "VALIDATION_PASSWORD_NO_SPECIAL",
        Region(_) => "VALIDATION_REGION",
        Language(_) => "VALIDATION_LANGUAGE",
        Custom(_) => "VALIDATION_FAILED",
    };
    ApiError {
        status: StatusCode::BAD_REQUEST,
        code,
        message: v.to_string(),
        request_id: None,
    }
}

/// Helper to convert from a `Result<T, DomainError>` to
/// `Result<T, ApiError>` in handler bodies.
pub type ApiResult<T> = Result<T, ApiError>;
