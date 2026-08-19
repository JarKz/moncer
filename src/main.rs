use std::time::Duration;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, post,
    web::{self, Json},
};
use entity::{category, transaction};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    QueryFilter, TransactionTrait,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateTransactionRequest {
    change_minor: i64,
    category_path: Option<Vec<String>>,
    reason: Option<String>,
    created_at_ms: Option<i64>,
}

#[derive(Serialize)]
struct TransactionResponse {
    id: i64,
    change_minor: i64,
    category_id: Option<i64>,
    reason: Option<String>,
    created_at: i64,
}

impl From<transaction::Model> for TransactionResponse {
    fn from(value: transaction::Model) -> Self {
        Self {
            id: value.id,
            change_minor: value.change_minor,
            category_id: value.category_id,
            reason: value.reason,
            created_at: value.created_at.and_utc().timestamp_millis(),
        }
    }
}

struct TransactionData {
    change_minor: i64,
    category_id: Option<i64>,
    reason: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<TransactionData> for transaction::ActiveModel {
    fn from(value: TransactionData) -> Self {
        use sea_orm::ActiveValue::*;
        Self {
            id: NotSet,
            change_minor: Set(value.change_minor),
            category_id: Set(value.category_id),
            reason: Set(value.reason),
            created_at: Set(value.created_at.naive_utc()),
        }
    }
}

#[derive(Serialize)]
struct ErrorMessage {
    error: ErrorCode,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
    ValidationFailed,
    CategoryNotFound,
    DatabaseError,
}

#[derive(Debug, derive_more::Display)]
enum ApplicationError {
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

    /// The category a user points to is absent in a database.
    #[display("Cannot find a category with name {category_name}")]
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
            ApplicationError::CategoryNotFound { .. } => {
                HttpResponse::NotFound().json(ErrorMessage {
                    error: ErrorCode::CategoryNotFound,
                    message,
                })
            }
        }
    }
}

fn validate_change(change_minor: i64) -> Result<(), ApplicationError> {
    if change_minor == 0 {
        return Err(ApplicationError::ValidationFailed {
            message: "Change must be not equal to 0.".to_owned(),
        });
    }

    Ok(())
}

fn validate_timestamp(
    timestamp_ms: i64,
) -> Result<chrono::DateTime<chrono::Utc>, ApplicationError> {
    /// 5 min.
    ///
    /// It was made for avoiding a possible time difference between client and server sides.
    const MAX_FUTURE_SKEW: i64 = 5 * 60 * 1000;

    if timestamp_ms < 0 {
        return Err(ApplicationError::ValidationFailed {
            message: "The \"created_at_ms\" field cannot be below 0.".to_owned(),
        });
    }

    if timestamp_ms > chrono::Utc::now().timestamp_millis() + MAX_FUTURE_SKEW {
        return Err(ApplicationError::ValidationFailed {
            message: "The \"created_at_ms\" field cannot be above than current time.".to_owned(),
        });
    }

    Ok(chrono::DateTime::from_timestamp_millis(timestamp_ms)
        .expect("The \"created_at_ms\" timestamp in must be valid"))
}

async fn find_category_by_path(
    category_path: &[String],
    database: &DatabaseConnection,
) -> Result<Vec<category::Model>, ApplicationError> {
    let mut parent_id = None;
    let mut actual_category_path: Vec<category::Model> = Vec::with_capacity(category_path.len());

    for category_name in category_path.iter().map(|name| name.trim()) {
        let parent_condition = match parent_id {
            Some(parent_id) => category::Column::ParentCategoryId.eq(parent_id),
            None => category::Column::ParentCategoryId.is_null(),
        };

        let category_model = category::Entity::find()
            .filter(parent_condition)
            .filter(category::Column::Name.eq(category_name))
            .one(database)
            .await
            .map_err(|_| ApplicationError::GenericDatabaseError)?;

        let model = match category_model {
            Some(model) => model,
            None => {
                return Err(ApplicationError::CategoryNotFound {
                    category_name: category_name.to_owned(),
                });
            }
        };

        parent_id = Some(model.id);
        actual_category_path.push(model);
    }

    Ok(actual_category_path)
}

async fn write_transaction(
    transaction_data: TransactionData,
    database: &DatabaseConnection,
) -> Result<transaction::Model, ApplicationError> {
    transaction::ActiveModel::from(transaction_data)
        .insert(database)
        .await
        .map_err(|_| ApplicationError::SpecificDatabaseError {
            reason: "Failed to add a transaction.".to_owned(),
        })
}

#[post("/transaction")]
async fn add_transaction(
    body: Json<CreateTransactionRequest>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    if let Err(e) = validate_change(body.change_minor) {
        return HttpResponse::from(e);
    }

    let created_at = match body.created_at_ms {
        Some(timestamp_ms) => match validate_timestamp(timestamp_ms) {
            Ok(val) => val,
            Err(e) => return HttpResponse::from(e),
        },
        None => chrono::Utc::now(),
    };

    let category_id = match &body.category_path {
        Some(category_path) => {
            if category_path.is_empty() || category_path.iter().any(|name| name.trim().is_empty()) {
                return HttpResponse::from(ApplicationError::ValidationFailed {
                    message: "The category path cannot be empty".to_owned(),
                });
            }

            match find_category_by_path(category_path, &database).await {
                Ok(categories) => categories.last().map(|category| category.id),
                Err(e) => return HttpResponse::from(e),
            }
        }
        None => None,
    };

    let transaction = match write_transaction(
        TransactionData {
            change_minor: body.change_minor,
            category_id,
            reason: body.reason.clone(),
            created_at,
        },
        &database,
    )
    .await
    {
        Ok(model) => model,
        Err(e) => return HttpResponse::from(e),
    };

    HttpResponse::Created().json(TransactionResponse::from(transaction))
}

#[derive(Deserialize)]
struct TransactionFilter {
    created_since_ms: Option<i64>,
    created_after_ms: Option<i64>,

    category_path: Option<Vec<String>>,

    #[serde(default)]
    exclude_subcategories: bool,

    limit: Option<u64>,
    offset: Option<u64>,
}

#[post("/transaction/search")]
async fn find_transaction(
    filter: Json<TransactionFilter>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    todo!();

    HttpResponse::Ok()
}

#[derive(Deserialize)]
#[serde(transparent)]
struct CreateCategoryRequest {
    category_path: Vec<String>,
}

#[derive(Serialize)]
struct CategoryResponse {
    id: i64,
    name: String,
    parent_category_id: Option<i64>,
}

impl From<category::Model> for CategoryResponse {
    fn from(value: category::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            parent_category_id: value.parent_category_id,
        }
    }
}

async fn ensure_category_path_exists(
    category_path: &[String],
    database: &DatabaseConnection,
) -> Result<Vec<CategoryResponse>, ApplicationError> {
    let mut parent_id = None;
    let mut response_category_path: Vec<CategoryResponse> = Vec::with_capacity(category_path.len());

    let txn = database
        .begin()
        .await
        .map_err(|_| ApplicationError::GenericDatabaseError)?;

    for category_name in category_path.iter().map(|name| name.trim()) {
        let parent_condition = match parent_id {
            Some(parent_id) => category::Column::ParentCategoryId.eq(parent_id),
            None => category::Column::ParentCategoryId.is_null(),
        };

        let category_model = category::Entity::find()
            .filter(parent_condition)
            .filter(category::Column::Name.eq(category_name))
            .one(&txn)
            .await
            .map_err(|_| ApplicationError::GenericDatabaseError)?;

        let model = match category_model {
            Some(model) => model,
            None => {
                use sea_orm::ActiveValue::*;

                category::ActiveModel {
                    id: NotSet,
                    name: Set(category_name.to_owned()),
                    parent_category_id: Set(parent_id),
                }
                .insert(&txn)
                .await
                .map_err(|_| ApplicationError::GenericDatabaseError)?
            }
        };

        parent_id = Some(model.id);
        response_category_path.push(model.into());
    }

    txn.commit()
        .await
        .map_err(|_| ApplicationError::GenericDatabaseError)?;

    Ok(response_category_path)
}

#[post("/category")]
async fn add_category(
    body: Json<CreateCategoryRequest>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    if body.category_path.is_empty() || body.category_path.iter().any(|name| name.trim().is_empty())
    {
        return HttpResponse::from(ApplicationError::ValidationFailed {
            message: "The category path cannot be empty".to_owned(),
        });
    }

    match ensure_category_path_exists(&body.category_path, &database).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::from(e),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host_url = match std::env::var("HOST_URL") {
        Ok(url) => url,
        Err(e) => {
            dbg!(e);
            eprintln!("Missing HOST_URL! Fallbacked to 127.0.0.1 address!");
            "127.0.0.1".to_owned()
        }
    };

    let host_port = match std::env::var("HOST_PORT")
        .ok()
        .and_then(|val| val.parse::<u16>().ok())
    {
        Some(port) => port,
        None => {
            eprintln!("Missing HOST_PORT! Fallbacked to 8080 port!");
            8080
        }
    };

    let database_url =
        std::env::var("DATABASE_URL").expect("The DATABASE_URL is required for the application.");

    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8));

    let database = web::Data::new(
        Database::connect(opt)
            .await
            .expect("The database must to be connected."),
    );

    HttpServer::new(move || {
        App::new().service(
            web::scope("/api")
                .app_data(database.clone())
                .service(add_transaction)
                .service(add_category),
        )
    })
    .bind((host_url, host_port))?
    .run()
    .await
}
