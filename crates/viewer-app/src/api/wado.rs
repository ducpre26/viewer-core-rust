//! UC-004: opaque WADO proxy. Local key images and session metadata prefetch
//! are deferred by the Step 11 scope decision; no DICOM parsing or JNI here.
use crate::auth::Auth;
use futures_util::TryStreamExt;
use std::{collections::BTreeMap, time::Duration};
use summer_web::{
    Router,
    aide::axum::routing::get,
    axum::{
        Extension,
        body::Body,
        extract::{Path, Request},
        http::{StatusCode, header},
        response::{IntoResponse, Response},
    },
};

#[derive(Clone, Copy)]
struct Operation {
    suffix: &'static str,
    accept: bool,
    thumbnail: bool,
    uri: bool,
}

// Explicit routes mirror WadoRsAPI, including the two-segment bulk-data route.
// A catch-all would expose upstream paths absent from the Java API.
const RS: &[(&str, bool)] = &[
    ("", false),
    ("/thumbnail", false),
    ("/metadata", false),
    ("/series", false),
    ("/series/{seriesUID}", false),
    ("/series/{seriesUID}/thumbnail", false),
    ("/series/{seriesUID}/metadata", false),
    ("/series/{seriesUID}/instances", false),
    ("/series/{seriesUID}/instances/{objectUID}", true),
    ("/series/{seriesUID}/instances/{objectUID}/thumbnail", false),
    ("/series/{seriesUID}/instances/{objectUID}/metadata", false),
    (
        "/series/{seriesUID}/instances/{objectUID}/frames/{frames}",
        true,
    ),
    (
        "/series/{seriesUID}/instances/{objectUID}/frames/{frames}/thumbnail",
        true,
    ),
    (
        "/series/{seriesUID}/instances/{objectUID}/{bulk}/{bulkData}",
        false,
    ),
];

#[derive(Clone)]
struct Wado {
    auth: Auth,
    client: reqwest::Client,
}

fn failure() -> Response {
    // WadoRsAPI catches unexpected exceptions (including null scoped sessions
    // and transport errors) and returns an empty 500, without an envelope.
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

fn missing_parameter(name: &str) -> Response {
    super::response::Error(
        StatusCode::INTERNAL_SERVER_ERROR,
        format!(
            "Required request parameter '{name}' for method parameter type String is not present"
        ),
        "INTERNAL_ERROR",
    )
    .into_response()
}

fn upstream_url(
    store: &str,
    operation: Operation,
    params: &BTreeMap<String, String>,
) -> Option<url::Url> {
    let store = if operation.uri {
        // Preserve Java's String.replace, including the trailing slash.
        store.replace("wado-rs", "")
    } else {
        store.to_owned()
    };
    let mut url = url::Url::parse(&store).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    {
        let mut path = url.path_segments_mut().ok()?;
        path.pop_if_empty();
        if operation.uri {
            path.push("wado");
        } else {
            path.push("studies").push(params.get("studyUID")?);
            for segment in operation.suffix.split('/').filter(|s| !s.is_empty()) {
                if let Some(key) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    path.push(params.get(key)?);
                } else {
                    path.push(segment);
                }
            }
        }
    }
    Some(url)
}

async fn retrieve(
    Extension(wado): Extension<Wado>,
    Extension(operation): Extension<Operation>,
    Path(params): Path<BTreeMap<String, String>>,
    request: Request,
) -> Response {
    // Spring's @RequestParam Map keeps the first value of duplicate keys.
    let mut query = BTreeMap::new();
    for (key, value) in url::form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes())
    {
        query.entry(key.into_owned()).or_insert(value.into_owned());
    }
    let study = if operation.uri {
        for required in ["studyUID", "seriesUID", "objectUID"] {
            if !query.contains_key(required) {
                return missing_parameter(required);
            }
        }
        &query["studyUID"]
    } else {
        &params["studyUID"]
    };
    let session = match wado.auth.session(&params["sessionID"]).await {
        Ok(Some(session)) if session.studies.contains(&Some(study.clone())) => session,
        _ => return failure(),
    };
    let Some(url) = session
        .store_url
        .as_deref()
        .and_then(|store| upstream_url(store, operation, &params))
    else {
        return failure();
    };
    let mut upstream = wado.client.get(url);
    if let Some(authorization) = &session.store_auth {
        upstream = upstream.header(header::AUTHORIZATION, authorization);
    }
    if operation.accept {
        upstream = upstream.header(
            header::ACCEPT,
            request
                .headers()
                .get(header::ACCEPT)
                .filter(|v| !v.is_empty())
                .cloned()
                .unwrap_or_else(|| header::HeaderValue::from_static("*/*")),
        );
    }
    if operation.uri {
        let mut forwarded = BTreeMap::new();
        for key in ["studyUID", "seriesUID", "objectUID"] {
            forwarded.insert(key, query[key].as_str());
        }
        forwarded.insert(
            "requestType",
            query
                .get("requestType")
                .filter(|v| !v.is_empty())
                .map_or("WADO", String::as_str),
        );
        // The controller passes null when absent; Feign omits contentType.
        if let Some(content_type) = query.get("contentType") {
            forwarded.insert("contentType", content_type);
        }
        upstream = upstream.query(&forwarded);
    } else if operation.thumbnail {
        // Java forwards all thumbnail query parameters, including unknown keys.
        upstream = upstream.query(&query);
    }
    let Ok(upstream) = upstream.send().await else {
        return failure();
    };
    // Feign methods returning Response bypass ErrorDecoder, including non-2xx.
    let status = upstream.status();
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .or_else(|| {
            // With no store media type, Spring selects a concrete Accept value;
            // the baseline's wildcard default is application/json. Remove quality
            // parameters and canonicalize parameter whitespace as Spring does.
            let accept = request.headers().get(header::ACCEPT)?.to_str().ok()?;
            let mut choices: Vec<_> = accept
                .split(',')
                .enumerate()
                .map(|(index, value)| {
                    let mut quality = 1.0_f32;
                    let parts: Vec<_> = value
                        .split(';')
                        .map(str::trim)
                        .filter(|part| {
                            if let Some(q) = part.strip_prefix("q=") {
                                quality = q.parse().unwrap_or(0.0);
                                false
                            } else {
                                true
                            }
                        })
                        .collect();
                    (quality, index, parts.join(";"))
                })
                .filter(|(quality, _, _)| *quality > 0.0)
                .collect();
            choices.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            let media = choices.first()?.2.as_str();
            if media == "*/*" || media == "application/*" {
                Some(header::HeaderValue::from_static("application/json"))
            } else {
                header::HeaderValue::from_str(media).ok()
            }
        })
        .or_else(|| Some(header::HeaderValue::from_static("application/json")));
    // Pull-based stream: no collection, producer task, or unbounded channel.
    // Dropping the downstream body drops the upstream response/connection.
    let stream = upstream
        .bytes_stream()
        .map_err(|_| std::io::Error::other("DICOM store stream failed"));
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    // Java's contentHeaders forwards only Content-Type. Transport, security
    // and CORS belong to this app; transfer syntax/boundary remain untouched.
    if let Some(value) = content_type {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    // Spring's reflected-file-download protection treats a numeric UID suffix
    // as an unknown file extension. Reproduce that generated header for DICOM
    // UID paths; never copy an upstream Content-Disposition.
    let last = if operation.uri {
        None
    } else if operation.suffix.is_empty() {
        params.get("studyUID")
    } else {
        operation.suffix.rsplit('/').next().and_then(|s| {
            s.strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
                .and_then(|key| params.get(key))
        })
    };
    if last.is_some_and(|s| {
        s.rsplit_once('.')
            .is_some_and(|(_, ext)| !ext.is_empty() && ext.bytes().all(|b| b.is_ascii_digit()))
    }) {
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_static("inline;filename=f.txt"),
        );
    }
    response
}

pub(super) fn router(auth: Auth) -> Router {
    let wado = Wado {
        auth,
        client: reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(60))
            .build()
            .expect("WADO HTTP client configuration"),
    };
    let mut router = Router::new();
    for &(suffix, accept) in RS {
        let operation = Operation {
            suffix,
            accept,
            thumbnail: suffix.ends_with("/thumbnail"),
            uri: false,
        };
        router = router.api_route(
            &format!("/v1/session/{{sessionID}}/wado-rs/studies/{{studyUID}}{suffix}"),
            get(retrieve).layer(Extension(operation)),
        );
    }
    router
        .api_route(
            "/v1/session/{sessionID}/wado-uri",
            get(retrieve).layer(Extension(Operation {
                suffix: "",
                accept: false,
                thumbnail: false,
                uri: true,
            })),
        )
        .layer(Extension(wado))
}

#[cfg(test)]
mod tests;
