use actix_web::{HttpResponse, Responder, delete, post, web};
use repository::{category, transaction};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use shared::{
    category_path::ValidCatedoryPath,
    change::NonZeroChange,
    error::ApplicationError,
    timestamp::{ValidTimestamp, valid_timestamp_start},
};

#[derive(Deserialize)]
struct CreateTransactionRequest {
    change_minor: NonZeroChange,
    category_path: Option<ValidCatedoryPath>,
    reason: Option<String>,
    created_at_ms: Option<ValidTimestamp>,
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

#[post("")]
async fn add_transaction(
    web::Json(body): web::Json<CreateTransactionRequest>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    let category_id = match &body.category_path {
        Some(category_path) => match category::find_by_path(category_path, &database).await {
            Ok(categories) => categories.last().map(|category| category.id),
            Err(e) => return HttpResponse::from(e),
        },
        None => None,
    };

    let transaction = match transaction::create(
        transaction::Data {
            change_minor: body.change_minor.into_inner(),
            category_id,
            reason: body.reason,
            created_at: body
                .created_at_ms
                .map(ValidTimestamp::into_inner)
                .unwrap_or(chrono::Utc::now()),
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

#[delete("/{id}")]
async fn delete_transaction(
    transaction_id: web::Path<i64>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    match transaction::remove(transaction_id.into_inner(), &database).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::from(e),
    }
}

#[derive(Deserialize)]
struct RequestTransactionFilter {
    /// Filter transactions by the period starts with.
    ///
    /// Inclusive.
    created_since_ms: Option<ValidTimestamp>,

    /// Filter transactions by the period ends with.
    ///
    /// Exclusive.
    created_before_ms: Option<ValidTimestamp>,

    /// Filter transactions by the category.
    category_path: Option<ValidCatedoryPath>,

    /// Whether to exclude subcategories of a selected category.
    #[serde(default)]
    exclude_subcategories: bool,

    /// Pick N elements from the result list.
    limit: Option<u64>,

    /// Skip N elements from the result list.
    offset: Option<u64>,
}

#[post("/search")]
async fn find_transaction(
    web::Json(filter): web::Json<RequestTransactionFilter>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    let created_since = filter
        .created_since_ms
        .map(ValidTimestamp::into_inner)
        .unwrap_or_else(|| {
            chrono::DateTime::from_timestamp_millis(valid_timestamp_start()).unwrap()
        });

    let created_before = filter
        .created_before_ms
        .map(ValidTimestamp::into_inner)
        .unwrap_or_else(chrono::Utc::now);

    if created_since > created_before {
        return HttpResponse::from(ApplicationError::ValidationFailed {
            message: "Selected an invalid period.".to_owned(),
        });
    }

    let category_id = match &filter.category_path {
        Some(category_path) => match category::find_by_path(category_path, &database).await {
            Ok(model) => model.last().map(|category| category.id),
            Err(e) => return HttpResponse::from(e),
        },
        None => None,
    };

    let transactions = match transaction::find_with_filter(
        transaction::Filter {
            created_since,
            created_before,
            category_id,
            exclude_subcategories: filter.exclude_subcategories,
            limit: filter.limit,
            offset: filter.offset,
        },
        &database,
    )
    .await
    {
        Ok(models) => models
            .into_iter()
            .map(TransactionResponse::from)
            .collect::<Vec<_>>(),
        Err(e) => return HttpResponse::from(e),
    };

    HttpResponse::Ok().json(transactions)
}

pub fn web_scope() -> actix_web::Scope {
    web::scope("/transaction")
        .service(add_transaction)
        .service(delete_transaction)
        .service(find_transaction)
}
