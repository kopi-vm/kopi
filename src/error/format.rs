use crate::error::{ErrorContext, KopiError};

pub fn format_error_chain(error: &KopiError) -> String {
    let context = ErrorContext::new(error);
    context.to_string()
}

/// Format error for display to user with colors and formatting (primarily for shim errors)
pub fn format_error_with_color(error: &KopiError, use_color: bool) -> String {
    let red = if use_color { "\x1b[31m" } else { "" };
    let yellow = if use_color { "\x1b[33m" } else { "" };
    let cyan = if use_color { "\x1b[36m" } else { "" };
    let reset = if use_color { "\x1b[0m" } else { "" };
    let bold = if use_color { "\x1b[1m" } else { "" };

    let context = ErrorContext::new(error);
    let mut output = String::new();

    // Error header
    output.push_str(&format!("{red}{bold}Error:{reset} {error}\n"));

    // Details
    if let Some(details) = &context.details {
        output.push_str(&format!("\n{details}\n"));
    }

    // Suggestions
    if let Some(suggestion) = &context.suggestion {
        output.push_str(&format!("\n{yellow}{bold}Suggestions:{reset}\n"));
        // Split suggestion by newlines and add cyan bullet points
        for line in suggestion.lines() {
            if !line.trim().is_empty() {
                output.push_str(&format!("{cyan}• {line}{reset}\n"));
            }
        }
    }

    // Always end with a reset to ensure no color bleeding
    if use_color && !output.is_empty() {
        output.push_str(reset);
    }

    output
}
