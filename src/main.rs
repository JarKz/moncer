use std::time::Duration;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, post,
    web::{self, Json},
};
use entity::{category, transaction};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    QueryFilter,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateTransactionRequest {
    change_minor: i64,
    category: Option<String>,
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

fn validate_change(change_minor: i64) -> Result<(), HttpResponse> {
    if change_minor == 0 {
        return Err(HttpResponse::UnprocessableEntity().json(ErrorMessage {
            error: ErrorCode::ValidationFailed,
            message: "Change must be not equal to 0.".to_owned(),
        }));
    }

    Ok(())
}

fn validate_timestamp(timestamp_ms: i64) -> Result<chrono::DateTime<chrono::Utc>, HttpResponse> {
    /// 5 min.
    const MAX_FUTURE_SKEW: i64 = 5 * 60 * 1000;

    if timestamp_ms < 0 {
        return Err(HttpResponse::UnprocessableEntity().json(ErrorMessage {
            error: ErrorCode::ValidationFailed,
            message: "The \"created_at_ms\" field cannot be below 0.".to_owned(),
        }));
    }

    if timestamp_ms > chrono::Utc::now().timestamp_millis() + MAX_FUTURE_SKEW {
        return Err(HttpResponse::UnprocessableEntity().json(ErrorMessage {
            error: ErrorCode::ValidationFailed,
            message: "The \"created_at_ms\" cannot be above than current time.".to_owned(),
        }));
    }

    Ok(chrono::DateTime::from_timestamp_millis(timestamp_ms)
        .expect("The \"created_at_ms\" timestamp in must be valid"))
}

async fn find_category_by_name(
    category_name: &str,
    database: &DatabaseConnection,
) -> Result<category::Model, HttpResponse> {
    let Ok(result) = category::Entity::find()
        .filter(category::Column::Name.eq(category_name))
        .one(database)
        .await
    else {
        return Err(HttpResponse::InternalServerError().json(ErrorMessage {
            error: ErrorCode::DatabaseError,
            message: "Failed to make an operation. Try again later.".to_owned(),
        }));
    };

    result.ok_or(HttpResponse::NotFound().json(ErrorMessage {
        error: ErrorCode::CategoryNotFound,
        message: format!("Cannot find an category with name \"{category_name}\"."),
    }))
}

async fn write_transaction(
    transaction_data: TransactionData,
    database: &DatabaseConnection,
) -> Result<transaction::Model, HttpResponse> {
    match transaction::ActiveModel::from(transaction_data)
        .insert(database)
        .await
    {
        Ok(model) => Ok(model),
        Err(_) => Err(HttpResponse::InternalServerError().json(ErrorMessage {
            error: ErrorCode::DatabaseError,
            message: "Failed to add a transaction. Try again later.".to_owned(),
        })),
    }
}

#[post("/transaction")]
async fn add_transaction(
    body: Json<CreateTransactionRequest>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    if let Err(e) = validate_change(body.change_minor) {
        return e;
    }

    let created_at = match body.created_at_ms {
        Some(timestamp_ms) => match validate_timestamp(timestamp_ms) {
            Ok(val) => val,
            Err(e) => return e,
        },
        None => chrono::Utc::now(),
    };

    let category_id = match &body.category {
        Some(category_name) => match find_category_by_name(category_name, &database).await {
            Ok(category) => Some(category.id),
            Err(e) => return e,
        },
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
        Err(e) => return e,
    };

    HttpResponse::Created().json(TransactionResponse::from(transaction))
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello!")
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
        .max_lifetime(Duration::from_secs(8))
        .set_schema_search_path("my_schema");

    let database = web::Data::new(
        Database::connect(opt)
            .await
            .expect("The database must to be connected."),
    );

    HttpServer::new(move || {
        App::new().service(hello).service(
            web::scope("/api")
                .app_data(database.clone())
                .service(add_transaction),
        )
    })
    .bind((host_url, host_port))?
    .run()
    .await
}
