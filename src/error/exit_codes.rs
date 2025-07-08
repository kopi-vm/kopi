use crate::error::KopiError;

pub fn get_exit_code(error: &KopiError) -> i32 {
    match error {
        KopiError::InvalidVersionFormat(_)
        | KopiError::InvalidConfig(_)
        | KopiError::ValidationError(_) => 2,

        KopiError::NoLocalVersion { .. } => 3,

        KopiError::JdkNotInstalled { .. } => 4,

        KopiError::ToolNotFound { .. } => 5,

        KopiError::PermissionDenied(_) => 13,

        KopiError::NetworkError(_) | KopiError::Http(_) | KopiError::MetadataFetch(_) => 20,

        KopiError::DiskSpaceError(_) => 28,

        KopiError::AlreadyExists(_) => 17,

        KopiError::KopiNotFound { .. } => 127, // Standard "command not found" exit code

        _ => 1,
    }
}

