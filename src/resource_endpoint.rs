use actix_http::Response as HttpResponse;
use actix_web::web;
use evmap::ReadHandle;

use crate::{get, image_processor, resource_processor, WebDavClient, WebDavResource};

#[get("")]
pub async fn list_resources(kv_reader: web::Data<ReadHandle<String, String>>) -> HttpResponse {
    let keys: Vec<String> = resource_processor::get_all(kv_reader.as_ref());

    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&keys).unwrap())
}

#[get("week")]
pub async fn list_this_week_resources(kv_reader: web::Data<ReadHandle<String, String>>) -> HttpResponse {
    let keys: Vec<String> = resource_processor::get_this_week_in_past(kv_reader.as_ref());

    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&keys).unwrap())
}

#[get("{resource_id}/{display_width}/{display_height}")]
pub async fn get_resource(resources_id: web::Path<(String, u32, u32)>, kv_reader: web::Data<ReadHandle<String, String>>, web_dav_client: web::Data<WebDavClient>) -> HttpResponse {
    let path_params = resources_id.0;
    let resource_id = path_params.0.as_str();
    let display_width = path_params.1;
    let display_height = path_params.2;

    let web_dav_resource = kv_reader.get_one(resource_id)
        .map(|value| value.to_string())
        .and_then(|resource_json_string| serde_json::from_str(resource_json_string.as_str()).ok());
    let orientation = web_dav_resource.clone().and_then(|web_dav_resource: WebDavResource| web_dav_resource.orientation);

    let resource_data = web_dav_resource
        .map(|web_dav_resource| web_dav_client.request_resource_data(&web_dav_resource))
        .and_then(|web_response| web_response.bytes().ok())
        .map(|resource_data| image_processor::optimize_image(
            resource_data.to_vec(),
            display_width,
            display_height,
            orientation,
        ));

    if let Some(resource_data) = resource_data {
        HttpResponse::Ok()
            .content_type("image/png")
            .body(resource_data)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

#[get("{resource_id}/{display_width}/{display_height}/base64")]
pub async fn get_resource_base64(resources_id: web::Path<(String, u32, u32)>, kv_reader: web::Data<ReadHandle<String, String>>, web_dav_client: web::Data<WebDavClient>) -> HttpResponse {
    // TODO request caching

    let path_params = resources_id.0;
    let resource_id = path_params.0.as_str();
    let display_width = path_params.1;
    let display_height = path_params.2;

    let web_dav_resource = kv_reader.get_one(resource_id)
        .map(|value| value.to_string())
        .and_then(|resource_json_string| serde_json::from_str(resource_json_string.as_str()).ok());
    let orientation = web_dav_resource.clone().and_then(|web_dav_resource: WebDavResource| web_dav_resource.orientation);

    let base64_image = web_dav_resource
        .map(|web_dav_resource| web_dav_client.request_resource_data(&web_dav_resource))
        .and_then(|web_response| web_response.bytes().ok())
        .map(|resource_data| image_processor::optimize_image(resource_data.to_vec(), display_width, display_height, orientation))
        .map(|scaled_image| base64::encode(&scaled_image))
        .map(|base64_string| format!("data:image/png;base64,{}", base64_string));

    if let Some(base64_image) = base64_image {
        HttpResponse::Ok()
            .content_type("plain/text")
            .body(base64_image)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

#[get("{resource_id}/metadata")]
pub async fn get_resource_metadata(resources_id: web::Path<String>, kv_reader: web::Data<ReadHandle<String, String>>) -> HttpResponse {
    let metadata = kv_reader.get_one(resources_id.as_str())
        .map(|value| value.to_string());

    if let Some(metadata) = metadata {
        HttpResponse::Ok()
            .content_type("application/json")
            .body(metadata)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

#[get("{resource_id}/description")]
pub async fn get_resource_metadata_description(resources_id: web::Path<String>, kv_reader: web::Data<ReadHandle<String, String>>) -> HttpResponse {
    let display_value = kv_reader.get_one(resources_id.as_str())
        .map(|value| value.to_string())
        .and_then(|resource_json_string| serde_json::from_str(resource_json_string.as_str()).ok())
        .map(resource_processor::build_display_value);

    if let Some(display_value) = display_value {
        HttpResponse::Ok()
            .content_type("plain/text")
            .body(display_value)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}
