use actix_web::{HttpResponse, Responder, get, post, web};
use repository::category;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use shared::category_path::ValidCatedoryPath;

#[derive(Deserialize)]
#[serde(transparent)]
struct CreateCategoryRequest {
    category_path: ValidCatedoryPath,
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

#[post("")]
async fn add_category(
    body: web::Json<CreateCategoryRequest>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    match category::ensure_path_exists(&body.category_path, &database).await {
        Ok(response) => HttpResponse::Ok().json(
            response
                .into_iter()
                .map(CategoryResponse::from)
                .collect::<Vec<_>>(),
        ),
        Err(e) => HttpResponse::from(e),
    }
}

#[get("")]
async fn get_categories(database: web::Data<DatabaseConnection>) -> impl Responder {
    let categories = match category::find_all(&database).await {
        Ok(models) => models
            .into_iter()
            .map(CategoryResponse::from)
            .collect::<Vec<_>>(),
        Err(e) => return HttpResponse::from(e),
    };

    HttpResponse::Found().json(categories)
}

#[get("/{id}")]
async fn get_category_by_id(
    category_id: web::Path<i64>,
    database: web::Data<DatabaseConnection>,
) -> impl Responder {
    match category::find_by_id(category_id.into_inner(), &database).await {
        Ok(model) => HttpResponse::Found().json(CategoryResponse::from(model)),
        Err(e) => HttpResponse::from(e),
    }
}

pub fn web_scope() -> actix_web::Scope {
    web::scope("/category")
        .service(add_category)
        .service(get_category_by_id)
        .service(get_categories)
}
