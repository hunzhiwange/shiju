use super::*;

pub fn start_variable_value_type_label(input_type: &str) -> &'static str {
    match input_type {
        "number" => "number",
        "checkbox" => "boolean",
        "file" => "file",
        "file-list" => "array[file]",
        _ => "string",
    }
}
