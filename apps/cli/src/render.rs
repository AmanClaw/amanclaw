use std::io::Write;

/// Print an LLM response to stdout.
pub fn print_response(text: &str) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "{text}").ok();
}

/// Print a thinking indicator to stderr.
pub fn print_thinking() {
    eprint!("Thinking...");
}

/// Clear the thinking indicator.
pub fn clear_thinking() {
    eprint!("\r            \r");
}

/// Print an error to stderr.
pub fn print_error(err: &anyhow::Error) {
    eprintln!("Error: {err:#}");
}

/// Print the interactive chat prompt.
pub fn print_prompt() {
    eprint!("you> ");
    std::io::stderr().flush().ok();
}
