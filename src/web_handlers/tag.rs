use std::collections::HashMap;

use actix_web::{error, web, HttpRequest, HttpResponse};
use sqlx::{MySqlPool, Row};

use crate::collection;
use crate::models::{AuthedUser, Tag};
use crate::web_handlers::get_param;

#[actix_web::get("/tags")]
async fn get_tags(pool: web::Data<MySqlPool>, _user: AuthedUser) -> actix_web::Result<web::Json<Vec<Tag>>> {
    let tags = sqlx::query_as::<_, Tag>("SELECT * FROM tags").fetch_all(pool.as_ref()).await.unwrap();
    Ok(web::Json(tags))
}

#[actix_web::get("/tag/{tag_id}")]
async fn get_tag(pool: web::Data<MySqlPool>, _user: AuthedUser, req: HttpRequest) -> actix_web::Result<web::Json<Tag>> {
    let tag_id: u64 = get_param(&req, "tag_id", "tag id must be a number!")?;

    // Query for the object and auto convert it.
    let query: Result<Tag, sqlx::Error> = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = ?").bind(tag_id).fetch_one(pool.as_ref()).await;

    // Check if the query was successful and return the tag,
    // if the tag could not be found, set the status code to 404.
    // Should a different kind of error occur, return an Internal Server Error (code: 500).
    match query {
        Ok(tag) => Ok(web::Json(tag)),
        Err(error) => Err(match error {
            sqlx::Error::RowNotFound => error::ErrorNotFound("tag not found!"),
            _ => error::ErrorInternalServerError(error),
        }),
    }
}

#[rustfmt::skip]
#[actix_web::put("/tag")]
async fn put_tag(pool: web::Data<MySqlPool>, _user: AuthedUser, tag: web::Json<Tag>) -> actix_web::Result<HttpResponse> {
    if tag.id != 0 {
        return Err(error::ErrorBadRequest("tag id must be 0!"));
    }

    // We need to make a transaction here because we want to make 2 queries that relate to each other.
    let mut tx = pool.as_ref().begin().await.map_err(error::ErrorInternalServerError)?;

    // First insert the object into the sql table...
    let insertion_query: Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> = sqlx::query("INSERT INTO tags (name,color,icon) VALUES (?,?,?)")
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(&tag.icon)
        .execute(&mut tx)
        .await;

    // ...then make sure it didn't fail.
    if let Err(error) = insertion_query {
        return Err(match error {
            sqlx::Error::Database(db_error) if db_error.message().starts_with("Duplicate entry") => error::ErrorConflict("there already is a tag with this name!"),
            _ => error::ErrorInternalServerError(error),
        });
    }

    // After that we need to get the autogenerated id from the table.
    let selection_query: Result<sqlx::mysql::MySqlRow, sqlx::Error> = sqlx::query("SELECT LAST_INSERT_ID()").fetch_one(&mut tx).await;

    // If we encounter an error then return status 500,
    // if not, extract the id from the query.
    let tag_id: u64 = selection_query.map_err(error::ErrorInternalServerError)?.try_get(0).unwrap();

    // Finally commit the changes to make them permanent
    tx.commit().await.map_err(error::ErrorInternalServerError)?;
    Ok(HttpResponse::Created().json::<HashMap<&str, u64>>(collection! {
        "tag_id" => tag_id
    }))
}

#[actix_web::post("/tag/{tag_id}")]
async fn update_tag(pool: web::Data<MySqlPool>, _user: AuthedUser, req: HttpRequest, tag: web::Json<Tag>) -> actix_web::Result<HttpResponse> {
    let tag_id: u64 = get_param(&req, "tag_id", "tag id must be a number!")?;
    if tag.id != tag_id {
        return Err(error::ErrorBadRequest("the tag ids don't match!"));
    }

    // Update the object in the sql table...
    let query: Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> = sqlx::query("UPDATE tags SET name = ?, color = ?, icon = ? WHERE id = ?")
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(&tag.icon)
        .bind(&tag.id)
        .execute(pool.as_ref())
        .await;

    // ...then make sure it didn't fail.
    let result = match query {
        Ok(result) => result,
        Err(error) => {
            return Err(match error {
                sqlx::Error::Database(db_error) if db_error.message().starts_with("Duplicate entry") => error::ErrorConflict("there already is a tag with this name!"),
                _ => error::ErrorInternalServerError(error),
            })
        }
    };

    // If nothing was changed, the tag didn't even exist!
    if result.rows_affected() == 0 {
        return Err(error::ErrorNotFound("tag not found!"));
    }

    Ok(HttpResponse::Ok().finish())
}

#[actix_web::delete("/tag/{tag_id}")]
async fn delete_tag(pool: web::Data<MySqlPool>, _user: AuthedUser, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let tag_id: u64 = get_param(&req, "tag_id", "tag id must be a number!")?;

    let query: Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> = sqlx::query("DELETE FROM tags WHERE id = ?").bind(&tag_id).execute(pool.as_ref()).await;

    // Get the query result or else return error 500.
    let query_result = query.map_err(error::ErrorInternalServerError)?;

    // If nothing was deleted, the tag didn't even exist!
    if query_result.rows_affected() == 0 {
        return Err(error::ErrorNotFound("tag not found!"));
    }

    Ok(HttpResponse::Ok().finish())
}
