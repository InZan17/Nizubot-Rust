use std::collections::BTreeMap;

use mlua::{IntoLua, Lua, Table};
use serde_json::{Map, Number};

use crate::Error;

fn describe_location(path: Vec<String>) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    Some(format!("Value is located at {}", path.join("->")))
}

pub fn lua_to_serde(lua_value: mlua::Value) -> Result<serde_json::Value, Error> {
    lua_to_serde_recursive(lua_value, vec![], vec![])
}

fn lua_to_serde_recursive(
    lua_value: mlua::Value,
    path: Vec<String>,
    mut found_tables: Vec<Table>,
) -> Result<serde_json::Value, Error> {
    Ok(match lua_value {
        mlua::Value::Nil => serde_json::Value::Null,
        mlua::Value::Boolean(value) => serde_json::Value::Bool(value),
        mlua::Value::Integer(integer) => {
            let number = integer as f64;
            let Some(number) = Number::from_f64(number) else {
                return Err(format!(
                    "Tried to store a number that's not finite: {number}. {}",
                    describe_location(path).unwrap_or_default()
                )
                .into());
            };
            serde_json::Value::Number(number)
        }
        mlua::Value::Number(number) => {
            let Some(number) = Number::from_f64(number) else {
                return Err(format!(
                    "Tried to store a number that's not finite: {number}. {}",
                    describe_location(path).unwrap_or_default()
                )
                .into());
            };
            serde_json::Value::Number(number)
        }
        mlua::Value::String(string) => match string.to_str() {
            Ok(string) => serde_json::Value::String(string.to_string()),
            Err(err) => {
                return Err(format!(
                    "Tried to store a Lua string that's not valid: {err}. {}",
                    describe_location(path).unwrap_or_default()
                )
                .into());
            }
        },
        mlua::Value::Table(table) => {
            if found_tables.contains(&table) {
                return Err(format!(
                    "Found table which is inside of itself. {}",
                    describe_location(path).unwrap_or_default()
                )
                .into());
            }

            found_tables.push(table.clone());

            let mut max_index = 0;
            let mut int_keys = 0;
            let mut string_keys = 0;

            for pair in table.pairs::<mlua::Value, mlua::Value>() {
                let (k, _) = pair?;
                match k {
                    mlua::Value::Integer(i) => {
                        if i <= 0 {
                            return Err(format!(
                                "Tried to store a Lua table with an unsupported number key: {i}. Please make sure the keys in the table are whole numbers larger than 0 or string values. {}",
                                describe_location(path).unwrap_or_default()
                            ).into());
                        };
                        let idx = i as usize;
                        int_keys += 1;
                        if idx > max_index {
                            max_index = idx;
                        }

                        if string_keys > 0 {
                            break;
                        }
                    }
                    mlua::Value::Number(n) => {
                        if n.fract() != 0.0 || n <= 0.0 {
                            return Err(format!(
                                "Tried to store a Lua table with an unsupported number key: {n}. Please make sure the keys in the table are whole numbers larger than 0 or string values. {}",
                                describe_location(path).unwrap_or_default()
                            ).into());
                        };
                        let idx = n as usize;
                        int_keys += 1;
                        if idx > max_index {
                            max_index = idx;
                        }

                        if string_keys > 0 {
                            break;
                        }
                    }
                    mlua::Value::String(_) => {
                        string_keys += 1;
                        if int_keys > 0 {
                            break;
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Type \"{}\" cannot be used as a key. {}",
                            k.type_name(),
                            describe_location(path).unwrap_or_default()
                        )
                        .into())
                    }
                }
            }

            if string_keys > 0 && int_keys > 0 {
                return Err(format!(
                    "Cannot store a table with both number keys and string keys. Please only use string keys or number keys. {}",
                    describe_location(path).unwrap_or_default()
                )
                .into());
            }

            if int_keys > 0 {
                if int_keys != max_index {
                    return Err(format!(
                        "Cannot store a table that is a sparse array. Please make it non-sparse, or turn the numbers into strings. {}",
                        describe_location(path).unwrap_or_default()
                    )
                    .into());
                }

                let mut array = Vec::with_capacity(int_keys);

                for (i, result) in table.sequence_values().into_iter().enumerate() {
                    let lua_value = result?;
                    let mut new_path = path.clone();
                    new_path.push(format!("[{}]", i + 1));
                    array.push(lua_to_serde_recursive(
                        lua_value,
                        new_path,
                        found_tables.clone(),
                    )?);
                }

                serde_json::Value::Array(array)
            } else {
                let mut map = Map::with_capacity(string_keys);
                for result in table.pairs::<mlua::String, mlua::Value>() {
                    let (key, lua_value) = result?;

                    let key = match key.to_str() {
                        Ok(string) => string.to_string(),
                        Err(err) => {
                            return Err(format!(
                                "Tried to store a Lua string that's not valid: {err}. {}",
                                describe_location(path).unwrap_or_default()
                            )
                            .into());
                        }
                    };

                    let mut new_path = path.clone();
                    new_path.push(format!("{key}"));
                    map.insert(
                        key,
                        lua_to_serde_recursive(lua_value, new_path, found_tables.clone())?,
                    );
                }
                serde_json::Value::Object(map)
            }
        }
        _ => {
            return Err(format!(
                "Type \"{}\" cannot be stored. {}",
                lua_value.type_name(),
                describe_location(path).unwrap_or_default()
            )
            .into())
        }
    })
}

pub fn serde_to_lua(serde_value: serde_json::Value, lua: &Lua) -> Result<mlua::Value, mlua::Error> {
    Ok(match serde_value {
        serde_json::Value::Null => mlua::Value::Nil,
        serde_json::Value::Bool(value) => mlua::Value::Boolean(value),
        serde_json::Value::Number(number) => {
            mlua::Value::Number(number.as_f64().expect("Please disable arbitrary_precision"))
        }
        serde_json::Value::String(string) => string.into_lua(lua)?,
        serde_json::Value::Array(values) => {
            let mut lua_values = Vec::with_capacity(values.len());
            for value in values.into_iter() {
                lua_values.push(serde_to_lua(value, lua)?);
            }

            lua_values.into_lua(lua)?
        }
        serde_json::Value::Object(map) => {
            let mut lua_values = BTreeMap::new();
            for (key, value) in map.into_iter() {
                lua_values.insert(key, serde_to_lua(value, lua)?);
            }

            lua_values.into_lua(lua)?
        }
    })
}
