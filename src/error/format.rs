use crate::error::{ErrorContext, KopiError};
use colored::Colorize;

pub fn format_error_chain(error: &KopiError) -> String {
    let context = ErrorContext::new(error);
    context.to_string()
}

/// Format error for display to user with colors and formatting (primarily for shim errors)
pub fn format_error_with_color(error: &KopiError, use_color: bool) -> String {
    // Control colored output globally
    colored::control::set_override(use_color);

    let context = ErrorContext::new(error);
    let mut output = String::new();

    // Error header
    output.push_str(&format!("{} {error}\n", "Error:".red().bold()));

    // Details
    if let Some(details) = &context.details {
        output.push_str(&format!("\n{details}\n"));
    }

    // Suggestions
    if let Some(suggestion) = &context.suggestion {
        output.push_str(&format!("\n{}\n", "Suggestions:".yellow().bold()));
        // Split suggestion by newlines and add cyan bullet points
        for line in suggestion.lines() {
            if !line.trim().is_empty() {
                output.push_str(&format!("{} {line}\n", "•".cyan()));
            }
        }
    }

    output
}
