use super::*;
use rusqlite::{types::ValueRef, ToSql};
use serde_json::Value;

pub(super) fn validate_identifier(input: &str) -> Result<(), DbError> {
    let starts_ok = input
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic())
        .unwrap_or(false);
    let chars_ok = input
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if !input.is_empty() && starts_ok && chars_ok {
        Ok(())
    } else {
        Err(DbError::InvalidInput(format!(
            "identifier `{}` must start with a letter and use only [A-Za-z0-9_]",
            input
        )))
    }
}

pub(super) fn validate_field_type(field_type: &str) -> Result<(), DbError> {
    let allowed = [
        "text",
        "integer",
        "real",
        "boolean",
        "date",
        "image",
        "single_select",
        "reference",
    ];
    if allowed.contains(&field_type) {
        Ok(())
    } else {
        Err(DbError::InvalidInput(format!(
            "unsupported field type: {}",
            field_type
        )))
    }
}

pub(super) fn sqlite_type_for(field_type: &str) -> &'static str {
    match field_type {
        "text" | "image" => "TEXT",
        "real" => "REAL",
        "integer" | "boolean" | "date" | "single_select" | "reference" => "INTEGER",
        _ => "TEXT",
    }
}

pub(super) fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

pub(super) fn override_text(current: Option<String>, template: Option<String>) -> Option<String> {
    if current == template {
        None
    } else {
        current
    }
}

pub(super) fn override_number(current: Option<f64>, template: Option<f64>) -> Option<f64> {
    match (current, template) {
        (Some(current_value), Some(template_value))
            if (current_value - template_value).abs() <= 0.001 =>
        {
            None
        }
        (current_value, template_value) if current_value == template_value => None,
        (current_value, _) => current_value,
    }
}

pub(super) fn is_required_value_empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(text)) => text.trim().is_empty(),
        _ => false,
    }
}

pub(super) fn to_sql_value(value: Option<&Value>, field_type: &str) -> Box<dyn ToSql> {
    match field_type {
        "integer" | "date" | "single_select" | "reference" => {
            let parsed = value.and_then(|item| {
                item.as_i64()
                    .or_else(|| item.as_str().and_then(|text| text.parse::<i64>().ok()))
            });
            Box::new(parsed as Option<i64>)
        }
        "real" => {
            let parsed = value.and_then(|item| {
                item.as_f64()
                    .or_else(|| item.as_str().and_then(|text| text.parse::<f64>().ok()))
            });
            Box::new(parsed as Option<f64>)
        }
        "boolean" => {
            let parsed = value.and_then(|item| {
                item.as_bool()
                    .or_else(|| item.as_i64().map(|n| n != 0))
                    .or_else(|| {
                        item.as_str().and_then(|text| match text {
                            "true" | "1" => Some(true),
                            "false" | "0" => Some(false),
                            _ => None,
                        })
                    })
            });
            Box::new(parsed.map(bool_to_i64) as Option<i64>)
        }
        _ => Box::new(value.and_then(Value::as_str).map(|item| item.to_string()) as Option<String>),
    }
}

pub(super) fn require_trimmed<'a>(field_name: &str, value: &'a str) -> Result<&'a str, DbError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DbError::InvalidInput(format!("{field_name} is required")));
    }
    Ok(trimmed)
}

pub(super) fn parse_group_ids(value: Option<String>) -> Vec<i64> {
    value
        .unwrap_or_default()
        .split(',')
        .filter_map(|item| item.parse::<i64>().ok())
        .collect()
}

pub(super) fn sqlite_value_to_json(value: ValueRef<'_>) -> Result<Value, rusqlite::Error> {
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::from(v),
        ValueRef::Real(v) => Value::from(v),
        ValueRef::Text(v) => Value::from(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(_) => Value::Null,
    })
}
