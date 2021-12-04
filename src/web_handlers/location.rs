use std::collections::HashMap;

use actix_web::{error, web, HttpRequest, HttpResponse};
use sqlx::{MySqlPool, Row};

use crate::collection;
use crate::models::{AuthedUser, Location};
use crate::web_handlers::get_param;

#[actix_web::get("/locations")]
async fn get_locations(pool: web::Data<MySqlPool>, _user: AuthedUser) -> actix_web::Result<web::Json<Vec<Location>>> {
    let locations = sqlx::query_as::<_, Location>("SELECT * FROM locations").fetch_all(pool.as_ref()).await.unwrap();
    Ok(web::Json(locations))
}

#[actix_web::get("/location/{location_id}")]
async fn get_location(pool: web::Data<MySqlPool>, _user: AuthedUser, req: HttpRequest) -> actix_web::Result<web::Json<Location>> {
    let location_id: u64 = get_param(&req, "location_id", "location id must be a number!")?;

    // Query for the object and auto convert it.
    let query: Result<Location, sqlx::Error> = sqlx::query_as::<_, Location>("SELECT * FROM locations WHERE id = ?")
        .bind(location_id)
        .fetch_one(pool.as_ref())
        .await;

    // Check if the query was successful and return the location,
    // if the location could not be found, set the status code to 404.
    // Should a different kind of error occur, return an Internal Server Error (code: 500).
    match query {
        Ok(location) => Ok(web::Json(location)),
        Err(error) => Err(match error {
            sqlx::Error::RowNotFound => error::ErrorNotFound("location not found!"),
            _ => error::ErrorInternalServerError(error),
        }),
    }
}

#[rustfmt::skip]
#[actix_web::put("/location")]
async fn put_location(pool: web::Data<MySqlPool>, _user: AuthedUser, location: web::Json<Location>) -> actix_web::Result<HttpResponse> {
    if location.id != 0 {
        return Err(error::ErrorBadRequest("location id must be 0!"));
    }

    // We need to make a transaction here because we want to make 2 queries that relate to each other.
    let mut tx = pool.as_ref().begin().await.map_err(error::ErrorInternalServerError)?;

    // First insert the object into the sql table...
    let insertion_query: Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> = sqlx::query("INSERT INTO locations (name,database_id) VALUES (?,?)")
        .bind(&location.name)
        .bind(&location.database)
        .execute(&mut tx)
        .await;

    // ...then make sure it didn't fail.
    if let Err(error) = insertion_query {
        return Err(match error {
            sqlx::Error::Database(db_error) if db_error.message().starts_with("Duplicate entry") => error::ErrorConflict("there already is a location with this name!"),
            sqlx::Error::Database(db_error) if db_error.message().starts_with("Cannot add or update a child row: a foreign key constraint fails") => error::ErrorNotFound("unknown database id!"),
            _ => error::ErrorInternalServerError(error),
        });
    }

    // After that we need to get the autogenerated id from the table.
    let selection_query: Result<sqlx::mysql::MySqlRow, sqlx::Error> = sqlx::query("SELECT LAST_INSERT_ID()").fetch_one(&mut tx).await;

    // If we encounter an error then return status 500,
    // if not, extract the id from the query.
    let location_id: u64 = selection_query.map_err(error::ErrorInternalServerError)?.get(0);

    // Finally commit the changes to make them permanent
    tx.commit().await.map_err(error::ErrorInternalServerError)?;
    Ok(HttpResponse::Created().json::<HashMap<&str, u64>>(collection! {
        "location_id" => location_id
    }))
}

#[rustfmt::skip]
#[actix_web::post("/location/{location_id}")]
async fn update_location(pool: web::Data<MySqlPool>, _user: AuthedUser, req: HttpRequest, location: web::Json<Location>) -> actix_web::Result<HttpResponse> {
    let location_id: u64 = get_param(&req, "location_id", "location id must be a number!")?;
    if location.id != location_id {
        return Err(error::ErrorBadRequest("the location ids don't match!"));
    }

    // Update the object in the sql table...
    let query: Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> = sqlx::query("UPDATE locations SET name = ?, database_id = ? WHERE id = ?")
        .bind(&location.name)
        .bind(&location.database)
        .bind(&location.id)
        .execute(pool.as_ref())
        .await;

    // ...then make sure it didn't fail.
    let result = match query {
        Ok(result) => result,
        Err(error) => {
            return Err(match error {
                sqlx::Error::Database(db_error) if db_error.message().starts_with("Duplicate entry") => error::ErrorConflict("there already is a location with this name!"),
                sqlx::Error::Database(db_error) if db_error.message().starts_with("Cannot add or update a child row: a foreign key constraint fails") => error::ErrorNotFound("unknown database id!"),
                _ => error::ErrorInternalServerError(error),
            })
        }
    };

    // If nothing was changed, the location didn't even exist!
    if result.rows_affected() == 0 {
        return Err(error::ErrorNotFound("location not found!"));
    }

    Ok(HttpResponse::Ok().finish())
}

#[actix_web::delete("/location/{location_id}")]
async fn delete_location(pool: web::Data<MySqlPool>, _user: AuthedUser, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let location_id: u64 = get_param(&req, "location_id", "location id must be a number!")?;

    let query: Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> = sqlx::query("DELETE FROM locations WHERE id = ?").bind(&location_id).execute(pool.as_ref()).await;

    // Get the query result or else return error 500.
    let query_result = query.map_err(error::ErrorInternalServerError)?;

    // If nothing was deleted, the location didn't even exist!
    if query_result.rows_affected() == 0 {
        return Err(error::ErrorNotFound("location not found!"));
    }

    Ok(HttpResponse::Ok().finish())
}
