use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub enum AppError {
    Sqlx(sqlx::Error),
    Io(std::io::Error),
    Tera(tera::Error),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
    Regex(regex::Error),
    NotFound,
    ProblemNotFound,
    InvalidProblemFormat,
    Unauthorized,
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Sqlx(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<tera::Error> for AppError {
    fn from(err: tera::Error) -> Self {
        AppError::Tera(err)
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(err: serde_yaml::Error) -> Self {
        AppError::Yaml(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Json(err)
    }
}

impl From<regex::Error> for AppError {
    fn from(err: regex::Error) -> Self {
        AppError::Regex(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Sqlx(ref err) => {
                eprintln!("SQL Error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", err))
            }
            AppError::Io(ref err) => {
                eprintln!("IO Error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("File system error: {}", err))
            }
            AppError::Tera(ref err) => {
                eprintln!("Tera Template Error: {:?}", err);
                eprintln!("Tera Error Details: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {} - Details: {:?}", err, err))
            }
            AppError::Yaml(ref err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("YAML parsing error: {}", err)),
            AppError::Json(ref err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("JSON parsing error: {}", err)),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            AppError::InvalidProblemFormat => (StatusCode::BAD_REQUEST, "Invalid problem format".to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::ProblemNotFound => (StatusCode::NOT_FOUND, "Problem not found".to_string()),
            AppError::Regex(ref err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Regex error: {}", err)),
        };
        (status, message).into_response()
    }
}

