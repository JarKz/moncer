use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorMessage {
    error: ErrorCode,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ValidationFailed,
    ElementNotFound,
    CategoryNotFound,
    DatabaseError,
}

#[derive(Debug, derive_more::Display)]
pub enum ApplicationError {
    /// When a database fails to perform any operation.
    ///
    /// Not matter what is the operation.
    #[display("Failed to perform an operation. Try again later.")]
    GenericDatabaseError,

    /// When a database fails to perform a well known operation.
    #[display("{reason}. Try again later.")]
    SpecificDatabaseError { reason: String },

    /// The validation layer of a user input found some mistakes.
    #[display("{message}")]
    ValidationFailed { message: String },

    #[display("Cannot find the requested element by name or id: {key}.")]
    ElementNotFound { key: String },

    /// The category a user points to is absent in a database.
    #[display("Cannot find a category with name {category_name}.")]
    CategoryNotFound { category_name: String },
}

impl std::error::Error for ApplicationError {}

impl From<ApplicationError> for HttpResponse {
    fn from(value: ApplicationError) -> Self {
        let message = value.to_string();
        match &value {
            ApplicationError::GenericDatabaseError => {
                HttpResponse::InternalServerError().json(ErrorMessage {
                    error: ErrorCode::DatabaseError,
                    message,
                })
            }
            ApplicationError::SpecificDatabaseError { .. } => HttpResponse::InternalServerError()
                .json(ErrorMessage {
                    error: ErrorCode::DatabaseError,
                    message,
                }),
            ApplicationError::ValidationFailed { .. } => {
                HttpResponse::UnprocessableEntity().json(ErrorMessage {
                    error: ErrorCode::ValidationFailed,
                    message,
                })
            }
            ApplicationError::ElementNotFound { .. } => {
                HttpResponse::NotFound().json(ErrorMessage {
                    error: ErrorCode::ElementNotFound,
                    message,
                })
            }
            ApplicationError::CategoryNotFound { .. } => {
                HttpResponse::NotFound().json(ErrorMessage {
                    error: ErrorCode::CategoryNotFound,
                    message,
                })
            }
        }
    }
}
