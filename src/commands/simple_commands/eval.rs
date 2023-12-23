use std::time::{SystemTime, UNIX_EPOCH};

use evalexpr::{context_map, EvalexprError, Value};

use crate::{Context, Error};

/// I will evaluate an expression!
#[poise::command(slash_command)]
pub async fn eval(
    ctx: Context<'_>,
    #[description = "The expression"] expression: String,
) -> Result<(), Error> {
    let mut context = context_map! {
        "sin" => Function::new(|argument| {
            if let Ok(int) = argument.as_int() {
                Ok(Value::Float((int as f64).sin()))
            } else if let Ok(float) = argument.as_float() {
                Ok(Value::Float(float.sin()))
            } else {
                Err(EvalexprError::expected_number(argument.clone()))
            }
        }),
        "cos" => Function::new(|argument| {
            if let Ok(int) = argument.as_int() {
                Ok(Value::Float((int as f64).cos()))
            } else if let Ok(float) = argument.as_float() {
                Ok(Value::Float(float.cos()))
            } else {
                Err(EvalexprError::expected_number(argument.clone()))
            }
        })
    }
    .unwrap(); // Do proper error handling here

    let value = evalexpr::eval_with_context_mut(&expression, &mut context)?;

    ctx.say(format!("{value}")).await?;
    Ok(())
}
