use std::io::{self, Write};

pub fn prompt_secret(field_name: &str) -> anyhow::Result<String> {
    let prompt = format!("Enter {field_name}: ");
    rpassword::prompt_password(prompt).map_err(Into::into)
}

pub fn print_success(message: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{message}")?;
    Ok(())
}
